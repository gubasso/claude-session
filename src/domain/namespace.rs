//! Naming the kernel namespace one terminal name is unique inside.
//!
//! A terminal name answers "which pane" only within the namespace that issued
//! it. A state tree bind-mounted into containers crosses that boundary, where
//! every devpts starts again at `/dev/pts/0` and the name stops being an
//! answer. This module turns a namespace link and a per-kernel discriminator
//! into one path component; reading both is `adapters::host`, and deciding
//! the path is `domain::paths` ([ADR-0107], [ADR-0109]).
//!
//! [ADR-0107]: ../../docs/decisions/ADR-0107-scope-a-terminal-to-its-namespace.md
//! [ADR-0109]: ../../docs/decisions/ADR-0109-discriminate-namespaces-across-kernels.md

use std::{ffi::OsStr, os::unix::ffi::OsStrExt as _, str::FromStr as _};

use sha2::{Digest as _, Sha256};

use crate::domain::identifier::Identifier;

/// Which kernel identity discriminated a namespace name.
///
/// Two rungs, in preference order. The machine identifier is stable across
/// reboots, which durable session directories need; the boot identifier is
/// the honest fallback, stranding per boot rather than colliding across
/// kernels ([ADR-0109]).
///
/// [ADR-0109]: ../../docs/decisions/ADR-0109-discriminate-namespaces-across-kernels.md
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DiscriminatorSource {
    /// The kernel's machine identifier.
    Machine,
    /// The kernel's boot identifier.
    Boot,
}

impl DiscriminatorSource {
    /// Returns the phrase a report uses for this rung.
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Machine => "its machine identifier",
            Self::Boot => "its boot identifier",
        }
    }
}

/// One per-kernel discriminator: the identity read, and which rung read it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Discriminator {
    source: DiscriminatorSource,
    value: String,
}

impl Discriminator {
    /// Wraps one rung's value, refusing an empty one.
    ///
    /// An empty discriminator would make every kernel agree again, which is
    /// the collision this type exists to prevent, so it names nothing and the
    /// caller falls to the next rung.
    pub(crate) fn new(source: DiscriminatorSource, value: &str) -> Option<Self> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return None;
        }
        Some(Self {
            source,
            value: trimmed.to_owned(),
        })
    }

    /// Returns which rung read this discriminator.
    pub(crate) const fn source(&self) -> DiscriminatorSource {
        self.source
    }

    /// Selects the kernel's discriminator from the ladder's two rungs.
    ///
    /// Machine identifier first, because it is stable across reboots, which
    /// durable session directories need; the boot identifier is the fallback
    /// for a kernel that carries none, honest at the cost of stranding per
    /// boot. An unreadable rung arrives as `None` and an empty one is refused
    /// by [`Self::new`], so where neither yields a value the ladder names
    /// nothing rather than falling back to a shared name ([ADR-0109]).
    ///
    /// [ADR-0109]: ../../docs/decisions/ADR-0109-discriminate-namespaces-across-kernels.md
    pub(crate) fn select(machine: Option<&str>, boot: Option<&str>) -> Option<Self> {
        machine
            .and_then(|value| Self::new(DiscriminatorSource::Machine, value))
            .or_else(|| boot.and_then(|value| Self::new(DiscriminatorSource::Boot, value)))
    }
}

/// Which namespace issued the name a rung reads.
///
/// One per rung rather than one for the ladder, because the two rungs read
/// identifiers the kernel scopes differently: `/dev/pts` is a devpts
/// superblock, which the mount namespace carries, while a session id belongs to
/// the process namespace. `--pid=host` is the reachable configuration where one
/// uniform choice would collide and these do not.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Kind {
    /// Owns the device names the controlling-terminal rung reads.
    Mount,
    /// Owns the session ids the session-leader rung reads.
    Pid,
}

impl Kind {
    /// Returns this namespace's name under `/proc/self/ns`.
    pub(crate) const fn procfs_name(self) -> &'static str {
        match self {
            Self::Mount => "mnt",
            Self::Pid => "pid",
        }
    }

    /// Returns the phrase a report uses for this namespace.
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Mount => "its mount namespace",
            Self::Pid => "its process namespace",
        }
    }
}

/// One namespace: the path component, and which namespace it names.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Namespace {
    id: Identifier,
    kind: Kind,
    discriminated_by: DiscriminatorSource,
}

impl Namespace {
    /// Borrows the validated path component.
    pub(crate) const fn id(&self) -> &Identifier {
        &self.id
    }

    /// Returns which kernel identity discriminated this name.
    pub(crate) const fn discriminated_by(&self) -> DiscriminatorSource {
        self.discriminated_by
    }

    /// Returns which namespace this names.
    pub(crate) const fn kind(&self) -> Kind {
        self.kind
    }

    /// Names a namespace from its `/proc/self/ns` link and the kernel's
    /// discriminator.
    ///
    /// The link reads `pid:[4026533427]`, and the discriminator and the whole
    /// link value are fingerprinted together rather than parsed: the inode is
    /// opaque, nothing reads it back, and a fixed-width tag is what keeps the
    /// result inside the identifier grammar no matter how long the kernel's
    /// spelling grows. The discriminator joins the preimage because the inode
    /// counters carry fixed initial values every kernel shares, so the link
    /// alone cannot tell a guest kernel from its host ([ADR-0109]). Returns
    /// `None` for an empty link, so a caller falls to the next rung rather
    /// than storing state under a name every namespace would share.
    ///
    /// [ADR-0109]: ../../docs/decisions/ADR-0109-discriminate-namespaces-across-kernels.md
    ///
    /// The width answers the collision rather than the field: a truncated
    /// digest collides, and two namespaces colliding here restore exactly the
    /// interleaving [ADR-0102] exists to prevent, silently, because the
    /// directories still look separate. Truncation is never injective, so a
    /// width buys a probability rather than a guarantee: the four bytes this
    /// replaced had a colliding pair, pinned in the tests below, while twelve
    /// put the birthday bound over every namespace a state tree could
    /// accumulate beneath any failure the wrapper already tolerates. That is
    /// twenty-four of the twenty-eight characters the grammar leaves beside the
    /// longest prefix and its separator, and the last four buy nothing.
    ///
    /// [ADR-0102]: ../../docs/decisions/ADR-0102-key-child-state-by-terminal.md
    pub(crate) fn from_link(
        kind: Kind,
        link: &OsStr,
        discriminator: &Discriminator,
    ) -> Option<Self> {
        if link.as_bytes().is_empty() {
            return None;
        }
        // A NUL between the two fields is what keeps the preimage injective:
        // neither a kernel identity line nor a link value can contain one, so
        // no pair of values can imitate another pair's concatenation.
        let mut preimage = Vec::with_capacity(discriminator.value.len() + 1 + link.len());
        preimage.extend_from_slice(discriminator.value.as_bytes());
        preimage.push(0);
        preimage.extend_from_slice(link.as_bytes());
        fingerprint(kind.procfs_name(), &preimage).map(|id| Self {
            id,
            kind,
            discriminated_by: discriminator.source(),
        })
    }
}

/// Turns one preimage into a fixed-width path component behind a prefix.
///
/// The digest-and-tag scheme every fingerprinted component shares, so the
/// width rationale above is decided once. The peer scope reuses it over the
/// bare link — its boot component already separates kernels, so the
/// discriminator has no question left to answer there ([ADR-0108]).
///
/// [ADR-0108]: ../../docs/decisions/ADR-0108-share-the-child-peer-registry-across-sessions.md
pub(crate) fn fingerprint(prefix: &str, preimage: &[u8]) -> Option<Identifier> {
    use std::fmt::Write as _;

    let digest: [u8; 32] = Sha256::digest(preimage).into();
    let tag = digest[..12].iter().fold(String::new(), |mut text, byte| {
        // Writing into a `String` cannot fail, and the fold is what keeps
        // this one allocation rather than one per byte.
        let _ = write!(text, "{byte:02x}");
        text
    });
    Identifier::from_str(&format!("{prefix}-{tag}")).ok()
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    fn machine() -> Discriminator {
        Discriminator::new(DiscriminatorSource::Machine, "machine-fixture")
            .expect("a value names a discriminator")
    }

    fn named(kind: Kind, value: &str) -> Option<Namespace> {
        Namespace::from_link(kind, &OsString::from(value), &machine())
    }

    #[test]
    fn a_namespace_link_becomes_one_path_component() {
        let namespace = named(Kind::Pid, "pid:[4026533427]").expect("a link names a namespace");
        assert_eq!(namespace.id().as_str(), "pid-a5b2c243a5ddb4385ea9bfc4");
        assert_eq!(namespace.kind(), Kind::Pid);
        assert_eq!(namespace.discriminated_by(), DiscriminatorSource::Machine);
    }

    /// Slice 032 acceptance: the link values two kernels agree on stop
    /// agreeing once each kernel's own identity joins the preimage.
    #[test]
    fn two_machines_never_share_a_component() {
        let guest = Discriminator::new(DiscriminatorSource::Machine, "guest-fixture")
            .expect("a value names a discriminator");
        let link = OsString::from("mnt:[4026531840]");
        let host =
            Namespace::from_link(Kind::Mount, &link, &machine()).expect("a link names a namespace");
        let shared =
            Namespace::from_link(Kind::Mount, &link, &guest).expect("a link names a namespace");
        assert_ne!(host.id().as_str(), shared.id().as_str());
    }

    /// Slice 032 acceptance: an empty or unreadable machine identifier falls
    /// to the boot identifier, and where neither rung yields a value the
    /// ladder names nothing rather than falling back to a shared name.
    #[test]
    fn an_empty_discriminator_names_nothing() {
        for machine in [None, Some(""), Some("   "), Some("\n")] {
            let fallen = Discriminator::select(machine, Some("boot-fixture"))
                .expect("the boot rung discriminates");
            assert_eq!(fallen.source(), DiscriminatorSource::Boot);
            assert!(Discriminator::select(machine, None).is_none());
            assert!(Discriminator::select(machine, Some("  ")).is_none());
        }
        let readable = Discriminator::select(Some("machine-fixture"), Some("boot-fixture"))
            .expect("the machine rung discriminates");
        assert_eq!(readable.source(), DiscriminatorSource::Machine);
    }

    #[test]
    fn the_component_is_the_same_every_time() {
        assert_eq!(
            named(Kind::Mount, "mnt:[4026533412]"),
            named(Kind::Mount, "mnt:[4026533412]")
        );
    }

    #[test]
    fn two_namespaces_never_collapse_onto_one_name() {
        let host = named(Kind::Pid, "pid:[4026531836]").expect("a link names a namespace");
        let container = named(Kind::Pid, "pid:[4026533427]").expect("a link names a namespace");
        assert_ne!(host.id().as_str(), container.id().as_str());
    }

    #[test]
    fn the_two_kinds_never_share_a_name() {
        // Same inode, different namespace: the prefix is what keeps a mount
        // namespace from answering for a process namespace.
        let mount = named(Kind::Mount, "4026531836").expect("a link names a namespace");
        let pid = named(Kind::Pid, "4026531836").expect("a link names a namespace");
        assert_ne!(mount.id().as_str(), pid.id().as_str());
    }

    /// A four-byte tag would put these two mount namespaces in one directory,
    /// which is the collision the component exists to prevent. Pinned by a
    /// pair colliding under the fixture discriminator, so a narrower digest
    /// cannot return unnoticed.
    #[test]
    fn two_namespaces_sharing_a_short_digest_still_name_two_directories() {
        let first = named(Kind::Mount, "mnt:[4026503496]").expect("a link names a namespace");
        let second = named(Kind::Mount, "mnt:[4026522390]").expect("a link names a namespace");
        assert_eq!(&first.id().as_str()[..12], &second.id().as_str()[..12]);
        assert_ne!(first.id().as_str(), second.id().as_str());
    }

    #[test]
    fn an_empty_link_names_nothing() {
        assert!(named(Kind::Pid, "").is_none());
    }

    #[test]
    fn every_component_is_a_valid_identifier() {
        for link in ["pid:[1]", "mnt:[4026533412]", "pid:[18446744073709551615]"] {
            let namespace = named(Kind::Pid, link).expect("a link names a namespace");
            assert!(Identifier::from_str(namespace.id().as_str()).is_ok());
        }
    }
}
