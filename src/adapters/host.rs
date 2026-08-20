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
