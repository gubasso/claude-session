//! Naming the kernel namespace one terminal name is unique inside.
//!
//! A terminal name answers "which pane" only within the namespace that issued
//! it. A state tree bind-mounted into containers crosses that boundary, where
//! every devpts starts again at `/dev/pts/0` and the name stops being an
//! answer. This module turns a namespace link into one path component; reading
//! the link is `adapters::terminal`, and deciding the path is `domain::paths`
//! ([ADR-0107]).
//!
//! [ADR-0107]: ../../docs/decisions/ADR-0107-scope-a-terminal-to-its-namespace.md

use std::{ffi::OsStr, os::unix::ffi::OsStrExt as _, str::FromStr as _};

use sha2::{Digest as _, Sha256};

use crate::domain::identifier::Identifier;

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
}

impl Namespace {
    /// Borrows the validated path component.
    pub(crate) const fn id(&self) -> &Identifier {
        &self.id
    }

    /// Returns which namespace this names.
    pub(crate) const fn kind(&self) -> Kind {
        self.kind
    }

    /// Names a namespace from the verbatim value of its `/proc/self/ns` link.
    ///
    /// The link reads `pid:[4026533427]`, and the whole value is fingerprinted
    /// rather than parsed: the inode is opaque, nothing reads it back, and a
    /// fixed-width tag is what keeps the result inside the identifier grammar
    /// no matter how long the kernel's spelling grows. Returns `None` for an
    /// empty link, so a caller falls to the next rung rather than storing state
    /// under a name every namespace would share.
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
    pub(crate) fn from_link(kind: Kind, link: &OsStr) -> Option<Self> {
        use std::fmt::Write as _;

        if link.as_bytes().is_empty() {
            return None;
        }
        let digest: [u8; 32] = Sha256::digest(link.as_bytes()).into();
        let tag = digest[..12].iter().fold(String::new(), |mut text, byte| {
            // Writing into a `String` cannot fail, and the fold is what keeps
            // this one allocation rather than one per byte.
            let _ = write!(text, "{byte:02x}");
            text
        });
        Identifier::from_str(&format!("{}-{tag}", kind.procfs_name()))
            .ok()
            .map(|id| Self { id, kind })
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    fn named(kind: Kind, value: &str) -> Option<Namespace> {
        Namespace::from_link(kind, &OsString::from(value))
    }

    #[test]
    fn a_namespace_link_becomes_one_path_component() {
        let namespace = named(Kind::Pid, "pid:[4026533427]").expect("a link names a namespace");
        assert_eq!(namespace.id().as_str(), "pid-a4b78f002f39fc2bdbf5f6ca");
        assert_eq!(namespace.kind(), Kind::Pid);
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

    /// A four-byte tag put these two mount namespaces in one directory, which
    /// is the collision the component exists to prevent. Pinned by their real
    /// spellings so a narrower digest cannot return unnoticed.
    #[test]
    fn two_namespaces_sharing_a_short_digest_still_name_two_directories() {
        let first = named(Kind::Mount, "mnt:[4026541606]").expect("a link names a namespace");
        let second = named(Kind::Mount, "mnt:[4026555244]").expect("a link names a namespace");
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
