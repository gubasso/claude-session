//! Naming the terminal one run's child state directory belongs to.
//!
//! This module is for turning what the operating system says about a run's
//! terminal into one path component. It performs no syscall — that is
//! `adapters::terminal` — and decides no path, which is `domain::paths`.
//!
//! The ladder is the one [ADR-0062] recorded, minus the two rungs
//! [ADR-0065] retired, and reinstated by [ADR-0102] for the child's own state
//! directory alone.
//!
//! A name here is unique only inside the namespace that issued it, so every
//! rung carries the namespace owning the identifier it read ([ADR-0107]).
//!
//! [ADR-0062]: ../../docs/decisions/ADR-0062-derive-the-group-from-the-controlling-terminal.md
//! [ADR-0065]: ../../docs/decisions/ADR-0065-retire-the-terminal-group.md
//! [ADR-0102]: ../../docs/decisions/ADR-0102-key-child-state-by-terminal.md
//! [ADR-0107]: ../../docs/decisions/ADR-0107-scope-a-terminal-to-its-namespace.md

use std::{ffi::OsStr, os::unix::ffi::OsStrExt as _, str::FromStr as _};

use sha2::{Digest as _, Sha256};

use crate::domain::{identifier::Identifier, namespace::Namespace};

/// Which rung of the ladder named this run's terminal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Source {
    /// The pane's own pseudo-terminal, which survives detach and reattach.
    Tty,
    /// The session leader, for a run with no controlling terminal.
    SessionLeader,
}

impl Source {
    /// Returns the phrase a report uses for this rung.
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Tty => "its controlling terminal",
            Self::SessionLeader => "the session it leads, having no controlling terminal",
        }
    }
}

/// One run's terminal: the path component, the namespace it is unique inside,
/// and the rung that named it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Terminal {
    id: Identifier,
    namespace: Namespace,
    source: Source,
}

impl Terminal {
    /// Borrows the validated path component.
    pub(crate) const fn id(&self) -> &Identifier {
        &self.id
    }

    /// Borrows the namespace this terminal's name is unique inside.
    pub(crate) const fn namespace(&self) -> &Namespace {
        &self.namespace
    }

    /// Returns the rung that named this terminal.
    pub(crate) const fn source(&self) -> Source {
        self.source
    }

    /// Names a terminal from the controlling terminal's device path.
    ///
    /// `/dev/pts/3` becomes `pts-3`, and a device the mapping cannot represent
    /// reversibly carries a fingerprint of its own bytes. Returns `None` when
    /// nothing survives the mapping or the result is too long to be an
    /// identifier, so the caller falls to the next rung instead of storing
    /// state under a name two different devices could share.
    ///
    /// The namespace is the mount namespace, which is what carries the devpts
    /// instance this device name was issued by.
    pub(crate) fn from_tty(namespace: Namespace, device: &OsStr) -> Option<Self> {
        sanitize(device.as_bytes()).map(|id| Self {
            id,
            namespace,
            source: Source::Tty,
        })
    }

    /// Names a terminal from the session leader and the time it started.
    ///
    /// The start time is what stops a recycled process id from inheriting an
    /// earlier session's directory, which a bare identifier cannot promise.
    ///
    /// The namespace is the process namespace, which is what issued the session
    /// id and the process ids `/proc` reported it against.
    pub(crate) fn from_session_leader(
        namespace: Namespace,
        sid: u32,
        started: u64,
    ) -> Option<Self> {
        Identifier::from_str(&format!("sid-{sid}-{started}"))
            .ok()
            .map(|id| Self {
                id,
                namespace,
                source: Source::SessionLeader,
            })
    }
}

/// Maps a device path onto the identifier grammar, or gives up.
///
/// Deliberately total and lossy: every byte outside the grammar becomes `-`,
/// and a value that still cannot be an identifier is refused rather than
/// truncated. Truncation is what would let two devices name one directory.
///
/// The mapping alone is not injective — `/dev/pts/3` and `/dev/pts-3` both
/// reach `pts-3`, as would any two devices differing only in case — so a
/// device outside the reversible domain carries a fingerprint of its own
/// bytes. Inside that domain, `/` is the only byte that becomes `-`, so the
/// clean name of the ordinary `/dev/pts/<n>` pane is preserved.
fn sanitize(device: &[u8]) -> Option<Identifier> {
    let trimmed = device.strip_prefix(b"/dev/").unwrap_or(device);
    let mapped: String = trimmed
        .iter()
        .map(|byte| {
            let lowered = byte.to_ascii_lowercase();
            if lowered.is_ascii_lowercase() || lowered.is_ascii_digit() || lowered == b'_' {
                char::from(lowered)
            } else {
                '-'
            }
        })
        .collect();
    let candidate = mapped.trim_matches(|character| character == '-' || character == '_');
    if reversible(trimmed, &mapped, candidate) {
        return Identifier::from_str(candidate).ok();
    }
    Identifier::from_str(&format!("{candidate}-{}", fingerprint(trimmed))).ok()
}

/// Reports whether the whole transformation loses nothing about this device.
///
/// Two conditions, because there are two lossy steps. The mapping is injective
/// only over the grammar's own bytes plus `/`, which is the one byte it
/// rewrites and which never appears in the output of any other byte; an
/// upper-case device or any punctuation shares its mapped form with some other
/// path. The trim is lossy over that domain too — `_` is a grammar byte, so
/// `/dev/_pts/3` and `/dev/pts/3` both reach `pts-3` once the leading one is
/// cut — so nothing may have been trimmed either.
fn reversible(device: &[u8], mapped: &str, candidate: &str) -> bool {
    let in_domain = device.iter().all(|byte| {
        byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'_' || *byte == b'/'
    });
    in_domain && candidate.len() == mapped.len()
}

/// Fingerprints a device path, so two that map alike still name two directories.
///
/// Sixty-four bits, which is a fingerprint rather than a proof: two devices on
/// one machine colliding here is not something to plan around, but it is not
/// impossible the way the reversible branch above is. Every device this
/// project has a real case for — `/dev/pts/<n>`, `/dev/tty<n>` — takes that
/// branch and never reaches this one.
fn fingerprint(device: &[u8]) -> String {
    use std::fmt::Write as _;

    let digest: [u8; 32] = Sha256::digest(device).into();
    digest[..8].iter().fold(String::new(), |mut text, byte| {
        // Writing into a `String` cannot fail, and the fold is what keeps this
        // one allocation rather than one per byte.
        let _ = write!(text, "{byte:02x}");
        text
    })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::domain::namespace::Kind;
    use std::ffi::OsString;

    fn space(kind: Kind, link: &str) -> Namespace {
        let machine = crate::domain::namespace::Discriminator::new(
            crate::domain::namespace::DiscriminatorSource::Machine,
            "machine-fixture",
        )
        .expect("a value names a discriminator");
        Namespace::from_link(kind, &OsString::from(link), &machine)
            .expect("a link names a namespace")
    }

    fn tty(value: &str) -> Option<Terminal> {
        Terminal::from_tty(
            space(Kind::Mount, "mnt:[4026531840]"),
            &OsString::from(value),
        )
    }

    fn leader(sid: u32, started: u64) -> Option<Terminal> {
        Terminal::from_session_leader(space(Kind::Pid, "pid:[4026531836]"), sid, started)
    }

    #[test]
    fn a_pseudo_terminal_becomes_one_path_component() {
        let terminal = tty("/dev/pts/3").expect("pts device names a terminal");
        assert_eq!(terminal.id().as_str(), "pts-3");
        assert_eq!(terminal.source(), Source::Tty);
    }

    /// Lower-casing is lossy, so the name carries the fingerprint that keeps
    /// `/dev/ttyS0` and a hypothetical `/dev/ttys0` apart.
    #[test]
    fn a_serial_console_keeps_its_own_name() {
        let serial = tty("/dev/ttyS0").expect("serial device");
        assert!(
            serial.id().as_str().starts_with("ttys0-"),
            "{}",
            serial.id().as_str()
        );
        assert_ne!(
            serial.id(),
            tty("/dev/ttys0").expect("lower-case device").id()
        );
    }

    /// The mapping sends `/` and `-` to one byte, and the trim cuts a leading
    /// `_` that is otherwise a grammar byte. Neither may let two devices land
    /// in one directory.
    #[test]
    fn two_devices_never_collapse_onto_one_name() {
        let names = ["/dev/pts/3", "/dev/pts-3", "/dev/_pts/3", "/dev/pts/3_"];
        let ids: Vec<String> = names
            .iter()
            .map(|name| {
                tty(name)
                    .expect("device names a terminal")
                    .id()
                    .as_str()
                    .to_owned()
            })
            .collect();
        for (index, left) in ids.iter().enumerate() {
            for right in ids.iter().skip(index + 1) {
                assert_ne!(left, right, "{names:?} collapsed onto {left}");
            }
        }
        // The ordinary pane still keeps the clean name, which is the whole
        // reason the reversible domain exists.
        assert_eq!(ids[0], "pts-3");
    }

    /// Two devices must never collapse onto one directory, so a name that
    /// cannot be an identifier falls to the next rung rather than being cut.
    #[test]
    fn an_over_long_device_is_refused_rather_than_truncated() {
        let long = format!("/dev/{}", "a".repeat(64));
        assert!(tty(&long).is_none());
    }

    #[test]
    fn a_device_with_nothing_usable_is_refused() {
        assert!(tty("/dev/").is_none());
        assert!(tty("///").is_none());
    }

    #[test]
    fn the_session_rung_carries_the_leader_and_its_start_time() {
        let terminal = leader(4242, 987_654).expect("session names one");
        assert_eq!(terminal.id().as_str(), "sid-4242-987654");
        assert_eq!(terminal.source(), Source::SessionLeader);
    }

    /// The same leader restarted is a different session, so the start time has
    /// to reach the name.
    #[test]
    fn a_recycled_process_id_does_not_inherit_the_earlier_directory() {
        let first = leader(4242, 1).expect("first");
        let second = leader(4242, 2).expect("second");
        assert_ne!(first.id(), second.id());
    }

    /// The whole point of the namespace axis: a container's first pane and the
    /// host's carry one device name, and must not carry one directory.
    #[test]
    fn one_device_in_two_namespaces_names_two_terminals() {
        let host = Terminal::from_tty(
            space(Kind::Mount, "mnt:[4026531840]"),
            &OsString::from("/dev/pts/0"),
        )
        .expect("host pane");
        let container = Terminal::from_tty(
            space(Kind::Mount, "mnt:[4026533412]"),
            &OsString::from("/dev/pts/0"),
        )
        .expect("container pane");
        assert_eq!(host.id(), container.id());
        assert_ne!(host.namespace().id(), container.namespace().id());
        assert_ne!(host, container);
    }

    /// The longest name each rung can produce still has room for the namespace
    /// beside it, because the two are separate path components rather than one
    /// identifier ([ADR-0107]).
    #[test]
    fn the_longest_name_of_each_rung_stays_an_identifier() {
        let longest = leader(4_194_304, 18_446_744_073_709_551_615).expect("longest leader");
        assert_eq!(longest.id().as_str(), "sid-4194304-18446744073709551615");
        assert_eq!(longest.namespace().id().as_str().len(), 28);
    }
}
