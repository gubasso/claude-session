//! Host identity reads: which kernel boot and namespaces this run is in.
//!
//! Everything here is a `procfs` read answering "where is this process
//! running", as distinct from `adapters::terminal`, which answers "which pane".
//! Each read returns `None` when `/proc` does not answer, because every caller
//! treats an unreadable identity as an unavailable rung rather than a failure
//! ([ADR-0107], [ADR-0108]).
//!
//! [ADR-0107]: ../../docs/decisions/ADR-0107-scope-a-terminal-to-its-namespace.md
//! [ADR-0108]: ../../docs/decisions/ADR-0108-share-the-child-peer-registry-across-sessions.md

use crate::domain::namespace::{Discriminator, Kind, Namespace};

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

/// Reads a process's start time from `procfs`, in clock ticks.
///
/// Field 22 of `/proc/<pid>/stat`, counted from the last `)` because the
/// second field is the executable name and may itself contain both spaces and
/// parentheses. Linux-only, which [ADR-0046] already is. Shared by the
/// session-leader rung's naming and the liveness judgment that re-asks it
/// ([ADR-0112]); `None` covers an absent process and an unreadable `/proc`
/// alike, so a caller that needs the difference tests liveness first.
///
/// [ADR-0046]: ../../docs/decisions/ADR-0046-support-linux-and-a-single-child-baseline.md
/// [ADR-0112]: ../../docs/decisions/ADR-0112-keep-only-the-session-proven-live.md
pub(crate) fn process_started(pid: u32) -> Option<u64> {
    let text = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let tail = text.rsplit_once(')')?.1;
    tail.split_whitespace().nth(19)?.parse().ok()
}

/// Names this run's controlling terminal, confirmed against the device itself.
///
/// Three steps, and the third is what the other two are for. Field 7 of
/// `/proc/self/stat` — `tty_nr` — states which pane this process belongs to,
/// and states it whatever has happened to standard input, output, and error;
/// `domain::terminal::device_path` maps that number onto the path the device
/// is published at; and the candidate is then confirmed by reading the node's
/// own device number back. A candidate that does not confirm names nothing,
/// so a mapping this kernel disagrees with costs the rung asking rather than
/// keying one pane's state to another pane's directory.
///
/// Asking `/dev/tty` for its name instead is the thing this exists not to do:
/// that node is the alias every process shares, it resolves back to itself,
/// and it exists whether or not any terminal does — one directory for every
/// pane, and a liveness question that could only ever answer yes. Opening the
/// alias remains the right predicate for whether a terminal is *there*, which
/// is `adapters::terminal`'s separate question.
pub(crate) fn controlling_terminal() -> Option<std::ffi::OsString> {
    use std::os::unix::fs::MetadataExt as _;

    let number = terminal_number()?;
    let candidate = crate::domain::terminal::device_path(number)?;
    let facts = std::fs::metadata(&candidate).ok()?;
    crate::domain::terminal::is_device(number, facts.rdev()).then(|| candidate.into())
}

/// Reads this process's controlling-terminal number, or `None` for no terminal.
///
/// Field 7 of `/proc/self/stat`, counted from the last `)` for the reason
/// [`process_started`] documents. The kernel writes `0` for a process with no
/// controlling terminal, which is an answer rather than a failure and reaches
/// the caller the same way an unreadable `/proc` does.
fn terminal_number() -> Option<u32> {
    let text = std::fs::read_to_string("/proc/self/stat").ok()?;
    let tail = text.rsplit_once(')')?.1;
    let number: u32 = tail.split_whitespace().nth(4)?.parse().ok()?;
    (number != 0).then_some(number)
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
