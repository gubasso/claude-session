//! Host identity reads: which kernel boot and namespaces this run is in.
//!
//! Everything here is a `procfs` read answering "where is this process
//! running", as distinct from `adapters::terminal`, which answers "is there a
//! person at a terminal to ask". Each read returns `None` when `/proc` does not
//! answer, because every caller treats an unreadable identity as an unavailable
//! fact rather than a failure ([ADR-0108], [ADR-0113]).
//!
//! [ADR-0108]: ../../docs/decisions/ADR-0108-share-the-child-peer-registry-across-sessions.md
//! [ADR-0113]: ../../docs/decisions/ADR-0113-key-a-session-to-its-running-agent.md

use crate::domain::{
    agent::Agent,
    namespace::{Discriminator, Kind, Namespace},
};

/// Reads one of this process's namespace links, discriminated by this kernel.
///
/// `/proc/self/ns/<kind>` is a magic link whose value — `pid:[4026533427]` —
/// identifies the namespace. Read rather than resolved: there is nothing behind
/// it to resolve, and the value is the identity. The kernel's discriminator
/// joins the derivation because two kernels agree on the link values of their
/// initial namespaces ([ADR-0109]); a run whose kernel names no identity at
/// all names no namespace either.
///
/// [ADR-0109]: ../../docs/decisions/ADR-0109-discriminate-namespaces-across-kernels.md
pub(crate) fn namespace(kind: Kind) -> Option<Namespace> {
    let discriminator = discriminator()?;
    let link = namespace_link(kind)?;
    Namespace::from_link(kind, &link, &discriminator)
}

/// Reads one of this process's namespace links, verbatim.
///
/// The bare value, for the one caller whose derivation deliberately leaves
/// the discriminator out: the peer scope's boot component already separates
/// kernels ([ADR-0108]).
///
/// [ADR-0108]: ../../docs/decisions/ADR-0108-share-the-child-peer-registry-across-sessions.md
pub(crate) fn namespace_link(kind: Kind) -> Option<std::ffi::OsString> {
    std::fs::read_link(format!("/proc/self/ns/{}", kind.procfs_name()))
        .ok()
        .map(std::path::PathBuf::into_os_string)
}

/// Reads this kernel's discriminator, machine identifier first.
///
/// The machine identifier is stable across reboots, which durable session
/// directories need; the boot identifier is the fallback for a kernel that
/// carries none, honest at the cost of stranding per boot ([ADR-0109]).
///
/// [ADR-0109]: ../../docs/decisions/ADR-0109-discriminate-namespaces-across-kernels.md
fn discriminator() -> Option<Discriminator> {
    Discriminator::select(machine_id().as_deref(), boot_id().as_deref())
}

/// Reads the kernel's machine identifier, trimmed.
///
/// `None` covers an unreadable file and an empty value, both of which some
/// container images ship; the caller falls to the boot identifier.
fn machine_id() -> Option<String> {
    let text = std::fs::read_to_string("/etc/machine-id").ok()?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.to_owned())
}

/// What `/proc/<pid>/stat` says about one process, as far as liveness needs it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Stat {
    exited: bool,
    started: u64,
}

impl Stat {
    /// Reports whether the kernel has stopped running this process and is only
    /// still listing it because nothing has reaped it.
    pub(crate) const fn exited(self) -> bool {
        self.exited
    }

    /// Returns the process's start time, in clock ticks since this kernel
    /// booted.
    pub(crate) const fn started(self) -> u64 {
        self.started
    }
}

/// Reads one process's run state and start time from `procfs`, in one read.
///
/// One read rather than two, because the pair has to describe one process: an
/// identifier reaped between separate reads is reissued by the same kernel, and
/// the answer would then join one process's state to another's start time.
///
/// The run state is field 3 and the start time is field 22, both counted from
/// the last `)` because the second field is the executable name and may itself
/// contain spaces and parentheses. `Z`, `X`, and `x` are the states of a
/// process that has already terminated: it is still listed, and answers
/// `kill(pid, 0)` exactly as a running process does, so the state is the only
/// thing that tells the two apart. Linux-only, which [ADR-0046] already is.
/// `None` covers an absent process and an unreadable `/proc` alike, so a caller
/// that needs the difference tests liveness first.
///
/// [ADR-0046]: ../../docs/decisions/ADR-0046-support-linux-and-a-single-child-baseline.md
pub(crate) fn process_stat(pid: u32) -> Option<Stat> {
    let text = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let tail = text.rsplit_once(')')?.1;
    let mut fields = tail.split_whitespace();
    let exited = matches!(fields.next()?, "Z" | "X" | "x");
    let started = fields.nth(18)?.parse().ok()?;
    Some(Stat { exited, started })
}

/// Reads a process's start time from `procfs`, in clock ticks.
///
/// The naming half of [`process_stat`], for the callers that ask about a
/// process they already know is running — this run itself, and its ancestors.
pub(crate) fn process_started(pid: u32) -> Option<u64> {
    process_stat(pid).map(Stat::started)
}

/// Names the agent this run is about to become.
///
/// The wrapper execs the child, so the process that will be the agent is this
/// one: its identifier and start time survive the exec unchanged, which is
/// what lets a directory named before the exec answer for the agent after it
/// ([ADR-0113]). The process namespace scopes the identifier, because that is
/// what issued it and what `/proc` reports it against.
///
/// [ADR-0113]: ../../docs/decisions/ADR-0113-key-a-session-to-its-running-agent.md
pub(crate) fn agent() -> Option<Agent> {
    let space = namespace(Kind::Pid)?;
    let pid = std::process::id();
    Agent::new(space, pid, process_started(pid)?)
}

/// Walks this process's ancestry, newest first, as identifier and start time.
///
/// What it is for is marking the reader's own row: a run of `session list` is
/// not an agent and never has a directory of its own, so the only honest
/// reading of "this session" is the agent this command is running inside. An
/// ancestor is exactly that, and the start time comes along because a matched
/// identifier alone would let a reissued one claim the mark.
///
/// The walk stops at the first unreadable parent and at the namespace's root,
/// so a truncated answer costs the mark and nothing else. `ppid` is field 4 of
/// `/proc/<pid>/stat`, counted from the last `)` for the reason
/// [`process_started`] documents.
pub(crate) fn lineage() -> Vec<(u32, u64)> {
    // The initial namespace tops out at `1`, and a bounded walk is what stops
    // a `/proc` that answers inconsistently from spinning here.
    const CEILING: usize = 64;

    let mut found = Vec::new();
    let mut pid = std::process::id();
    for _ in 0..CEILING {
        let Some(started) = process_started(pid) else {
            break;
        };
        found.push((pid, started));
        match parent(pid) {
            Some(next) if next != 0 && next != pid => pid = next,
            _ => break,
        }
    }
    found
}

/// Reads one process's parent identifier from `procfs`.
fn parent(pid: u32) -> Option<u32> {
    let text = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let tail = text.rsplit_once(')')?.1;
    tail.split_whitespace().nth(1)?.parse().ok()
}

/// Reads the kernel's boot identifier, trimmed.
///
/// A fresh random UUID per kernel boot, which makes it the one identity two
/// kernels can never share — the initial-namespace inodes are compiled
/// constants and identical everywhere ([ADR-0108]). `None` covers an
/// unreadable `/proc` and a value that is somehow empty.
///
/// [ADR-0108]: ../../docs/decisions/ADR-0108-share-the-child-peer-registry-across-sessions.md
pub(crate) fn boot_id() -> Option<String> {
    let text = std::fs::read_to_string("/proc/sys/kernel/random/boot_id").ok()?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.to_owned())
}
