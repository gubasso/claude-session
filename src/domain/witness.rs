//! The recorded preimage of one terminal's name, and its liveness verdict.
//!
//! The name a session directory carries is deliberately lossy, so nothing can
//! recover from it which device or session leader it stood for. This module
//! owns the record a launch writes so a later run can re-ask the naming
//! question — the witness — and the pure judgment over it ([ADR-0110],
//! [ADR-0111]). It performs no syscall: gathering the observations is the
//! session service's, and deciding the record's path is `domain::paths`.
//!
//! [ADR-0110]: ../../docs/decisions/ADR-0110-record-the-terminal-witness-at-launch.md
//! [ADR-0111]: ../../docs/decisions/ADR-0111-collect-only-the-provably-dead-session.md

use crate::domain::{
    identifier::Identifier,
    namespace::Kind,
    terminal::{Source, Terminal, Witness},
};

/// The record version this module writes, and the only one it reads.
///
/// A marker carrying any other version is judged unknown rather than parsed
/// optimistically: a future field could change what liveness means, and
/// unknown is never collected.
const VERSION: u32 = 1;

/// One witness as the marker file records it.
///
/// A separate type from [`Witness`] because the record is a JSON document and
/// the device path is an OS byte string: a device outside UTF-8 has no honest
/// spelling here, so building the record refuses instead of guessing, and the
/// session stays unknown rather than misjudged.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(tag = "rung", rename_all = "kebab-case")]
pub(crate) enum Recorded {
    /// The controlling terminal's device path.
    Tty { device: String },
    /// The session leader and its start time in clock ticks.
    SessionLeader { sid: u32, started: u64 },
}

impl Recorded {
    /// Returns the rung this record witnesses.
    pub(crate) const fn source(&self) -> Source {
        match self {
            Self::Tty { .. } => Source::Tty,
            Self::SessionLeader { .. } => Source::SessionLeader,
        }
    }

    /// Returns the namespace kind whose component scopes this record.
    pub(crate) const fn namespace_kind(&self) -> Kind {
        match self.source() {
            Source::Tty => Kind::Mount,
            Source::SessionLeader => Kind::Pid,
        }
    }
}

/// The marker a launch writes beside the session directory it names.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub(crate) struct Marker {
    version: u32,
    #[serde(flatten)]
    witness: Recorded,
    namespace: Identifier,
    /// The kernel boot the leader rung's start time counts from. Recorded for
    /// both rungs, judged only by the leader rung.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    boot: Option<String>,
}

impl Marker {
    /// Builds the marker one launch records for its terminal.
    ///
    /// `None` when the device path is not valid UTF-8, which JSON cannot carry
    /// byte-exactly; the caller logs the degradation and writes nothing, so
    /// the session is judged unknown rather than by a lossy spelling.
    pub(crate) fn from_terminal(terminal: &Terminal, boot: Option<String>) -> Option<Self> {
        let witness = match terminal.witness() {
            Witness::Tty { device } => Recorded::Tty {
                device: String::from_utf8(device.clone()).ok()?,
            },
            Witness::SessionLeader { sid, started } => Recorded::SessionLeader {
                sid: *sid,
                started: *started,
            },
        };
        Some(Self {
            version: VERSION,
            witness,
            namespace: terminal.namespace().id().clone(),
            boot,
        })
    }

    /// Borrows the recorded witness.
    pub(crate) const fn witness(&self) -> &Recorded {
        &self.witness
    }

    /// Borrows the namespace component the record is scoped by.
    pub(crate) const fn namespace(&self) -> &Identifier {
        &self.namespace
    }

    /// Serializes the marker as its durable document.
    pub(crate) fn to_bytes(&self) -> Option<Vec<u8>> {
        let mut bytes = serde_json::to_vec(self).ok()?;
        bytes.push(b'\n');
        Some(bytes)
    }

    /// Reads a marker back, refusing any version this module does not write.
    pub(crate) fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let marker: Self = serde_json::from_slice(bytes).ok()?;
        (marker.version == VERSION).then_some(marker)
    }
}

/// What this run observed about a recorded session leader.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LeaderObservation {
    /// No process carries the recorded id.
    Absent,
    /// A process carries the id, with this start time.
    Running { started: u64 },
    /// A process carries the id but its start time could not be read.
    Unreadable,
}

/// The facts a judgment needs, gathered by the caller for one marker.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Observed {
    /// This kernel's component for the marker's namespace kind.
    pub(crate) namespace: Option<Identifier>,
    /// This kernel's boot identifier.
    pub(crate) boot: Option<String>,
    /// Whether the recorded device path exists; `None` when unobservable.
    pub(crate) device_present: Option<bool>,
    /// What the recorded session leader's id names now.
    pub(crate) leader: LeaderObservation,
}

/// One session directory's liveness.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Verdict {
    /// The recorded terminal still exists; never collected.
    Live,
    /// The recorded terminal is provably gone; the only collectable verdict.
    Dead,
    /// Out of scope or undecidable; never collected ([ADR-0111]).
    ///
    /// [ADR-0111]: ../../docs/decisions/ADR-0111-collect-only-the-provably-dead-session.md
    Unknown,
}

impl Verdict {
    /// Returns the word a report uses for this verdict.
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Live => "live",
            Self::Dead => "dead",
            Self::Unknown => "unknown",
        }
    }
}

/// Judges one marker against what this run observed.
///
/// A marker whose namespace component is not this run's is undecidable: the
/// name it witnesses was issued somewhere this kernel cannot look, so absence
/// of its device or process proves nothing ([ADR-0111]). Inside scope the tty
/// rung is live while its device exists — boot is ignored deliberately,
/// because a reopened slot after reboot is the same slot — and the leader rung
/// is live while its process id and start time match under the recorded boot,
/// dead under a foreign boot, and undecidable when either boot is unreadable.
///
/// [ADR-0111]: ../../docs/decisions/ADR-0111-collect-only-the-provably-dead-session.md
pub(crate) fn judge(marker: &Marker, observed: &Observed) -> Verdict {
    let Some(current) = &observed.namespace else {
        return Verdict::Unknown;
    };
    if current != &marker.namespace {
        return Verdict::Unknown;
    }
    match &marker.witness {
        Recorded::Tty { .. } => match observed.device_present {
            Some(true) => Verdict::Live,
            Some(false) => Verdict::Dead,
            None => Verdict::Unknown,
        },
        Recorded::SessionLeader { started, .. } => match (&marker.boot, &observed.boot) {
            (Some(recorded), Some(current)) if recorded != current => Verdict::Dead,
            (Some(_), Some(_)) => match observed.leader {
                LeaderObservation::Running { started: now } if now == *started => Verdict::Live,
                // A recycled id is as gone as an absent one: the start time is
                // what tells this process from the one the record witnessed.
                LeaderObservation::Absent | LeaderObservation::Running { .. } => Verdict::Dead,
                LeaderObservation::Unreadable => Verdict::Unknown,
            },
            _ => Verdict::Unknown,
        },
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::domain::namespace::{Discriminator, DiscriminatorSource, Namespace};
    use std::ffi::OsString;

    fn space(kind: Kind, link: &str) -> Namespace {
        let machine = Discriminator::new(DiscriminatorSource::Machine, "machine-fixture")
            .expect("a value names a discriminator");
        Namespace::from_link(kind, &OsString::from(link), &machine)
            .expect("a link names a namespace")
    }

    fn tty_marker() -> Marker {
        let terminal = Terminal::from_tty(
            space(Kind::Mount, "mnt:[4026531840]"),
            &OsString::from("/dev/pts/3"),
        )
        .expect("pts device names a terminal");
        Marker::from_terminal(&terminal, Some("boot-fixture".to_owned())).expect("utf-8 device")
    }

    fn leader_marker() -> Marker {
        let terminal =
            Terminal::from_session_leader(space(Kind::Pid, "pid:[4026531836]"), 4242, 987_654)
                .expect("session names one");
        Marker::from_terminal(&terminal, Some("boot-fixture".to_owned())).expect("leader records")
    }

    fn observed(marker: &Marker) -> Observed {
        Observed {
            namespace: Some(marker.namespace().clone()),
            boot: Some("boot-fixture".to_owned()),
            device_present: Some(true),
            leader: LeaderObservation::Running { started: 987_654 },
        }
    }

    #[test]
    fn a_marker_roundtrips_through_its_document() {
        for marker in [tty_marker(), leader_marker()] {
            let bytes = marker.to_bytes().expect("a marker serializes");
            assert_eq!(Marker::from_bytes(&bytes), Some(marker));
        }
    }

    #[test]
    fn a_foreign_version_is_not_read() {
        let bytes = tty_marker().to_bytes().expect("a marker serializes");
        let raised = String::from_utf8(bytes)
            .expect("json is text")
            .replace("\"version\":1", "\"version\":2");
        assert_eq!(Marker::from_bytes(raised.as_bytes()), None);
    }

    #[test]
    fn garbage_is_not_a_marker() {
        assert_eq!(Marker::from_bytes(b"not json"), None);
        assert_eq!(Marker::from_bytes(b"{}"), None);
    }

    #[test]
    fn a_device_outside_utf8_records_nothing() {
        use std::os::unix::ffi::OsStringExt as _;
        let terminal = Terminal::from_tty(
            space(Kind::Mount, "mnt:[4026531840]"),
            &OsString::from_vec(b"/dev/pts/\xFF3".to_vec()),
        )
        .expect("the fingerprint branch names it");
        assert_eq!(Marker::from_terminal(&terminal, None), None);
    }

    #[test]
    fn a_present_device_in_scope_is_live() {
        let marker = tty_marker();
        assert_eq!(judge(&marker, &observed(&marker)), Verdict::Live);
    }

    #[test]
    fn an_absent_device_in_scope_is_dead() {
        let marker = tty_marker();
        let facts = Observed {
            device_present: Some(false),
            ..observed(&marker)
        };
        assert_eq!(judge(&marker, &facts), Verdict::Dead);
    }

    #[test]
    fn an_unobservable_device_is_unknown() {
        let marker = tty_marker();
        let facts = Observed {
            device_present: None,
            ..observed(&marker)
        };
        assert_eq!(judge(&marker, &facts), Verdict::Unknown);
    }

    /// The tty rung ignores boot deliberately: a reopened slot after reboot is
    /// the same slot ([ADR-0111]).
    ///
    /// [ADR-0111]: ../../../docs/decisions/ADR-0111-collect-only-the-provably-dead-session.md
    #[test]
    fn the_tty_rung_survives_a_reboot() {
        let marker = tty_marker();
        let facts = Observed {
            boot: Some("another-boot".to_owned()),
            ..observed(&marker)
        };
        assert_eq!(judge(&marker, &facts), Verdict::Live);
    }

    #[test]
    fn a_foreign_namespace_is_unknown_not_dead() {
        for marker in [tty_marker(), leader_marker()] {
            let facts = Observed {
                namespace: Some("mnt-000000000000000000000000".parse().expect("identifier")),
                ..observed(&marker)
            };
            assert_eq!(judge(&marker, &facts), Verdict::Unknown);
        }
    }

    #[test]
    fn an_unreadable_namespace_is_unknown() {
        for marker in [tty_marker(), leader_marker()] {
            let facts = Observed {
                namespace: None,
                ..observed(&marker)
            };
            assert_eq!(judge(&marker, &facts), Verdict::Unknown);
        }
    }

    #[test]
    fn a_matching_leader_in_scope_is_live() {
        let marker = leader_marker();
        assert_eq!(judge(&marker, &observed(&marker)), Verdict::Live);
    }

    #[test]
    fn an_absent_leader_is_dead() {
        let marker = leader_marker();
        let facts = Observed {
            leader: LeaderObservation::Absent,
            ..observed(&marker)
        };
        assert_eq!(judge(&marker, &facts), Verdict::Dead);
    }

    /// The start time is what stops a recycled process id from keeping an
    /// earlier session alive.
    #[test]
    fn a_recycled_leader_id_is_dead() {
        let marker = leader_marker();
        let facts = Observed {
            leader: LeaderObservation::Running { started: 1 },
            ..observed(&marker)
        };
        assert_eq!(judge(&marker, &facts), Verdict::Dead);
    }

    /// No process survives its kernel, so a foreign boot needs no process
    /// check at all.
    #[test]
    fn a_leader_under_a_foreign_boot_is_dead() {
        let marker = leader_marker();
        let facts = Observed {
            boot: Some("another-boot".to_owned()),
            leader: LeaderObservation::Running { started: 987_654 },
            ..observed(&marker)
        };
        assert_eq!(judge(&marker, &facts), Verdict::Dead);
    }

    /// Either boot missing, or a start time that cannot be read, leaves the
    /// question open, and open questions are kept rather than collected.
    #[test]
    fn an_undecidable_leader_is_unknown() {
        let marker = leader_marker();
        let unreadable = Observed {
            leader: LeaderObservation::Unreadable,
            ..observed(&marker)
        };
        assert_eq!(judge(&marker, &unreadable), Verdict::Unknown);
        let bootless = Observed {
            boot: None,
            ..observed(&marker)
        };
        assert_eq!(judge(&marker, &bootless), Verdict::Unknown);
        let unrecorded = Marker {
            boot: None,
            ..leader_marker()
        };
        assert_eq!(judge(&unrecorded, &observed(&unrecorded)), Verdict::Unknown);
    }
}
