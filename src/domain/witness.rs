//! The recorded preimage of one terminal's name, and its liveness verdict.
//!
//! The name a session directory carries is deliberately lossy, so nothing can
//! recover from it which device or session leader it stood for. This module
//! owns the record a launch writes so a later run can re-ask the naming
//! question — the witness — and the pure judgment over it ([ADR-0110],
//! [ADR-0112]). It performs no syscall: gathering the observations is the
//! session service's, and deciding the record's path is `domain::paths`. The
//! judgment answers with the ground it stands on, and the verdict is that
//! ground's projection, so a report can say why without re-deriving it.
//!
//! [ADR-0110]: ../../docs/decisions/ADR-0110-record-the-terminal-witness-at-launch.md
//! [ADR-0112]: ../../docs/decisions/ADR-0112-keep-only-the-session-proven-live.md

use crate::domain::{
    identifier::Identifier,
    namespace::Kind,
    terminal::{Source, Terminal, Witness},
};

/// The record version this module writes, and the only one it reads.
///
/// A marker carrying any other version is judged unknown rather than parsed
/// optimistically: a future field could change what liveness means, and a
/// record this module cannot read is not one it may claim to have judged.
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

/// The device path that names no pane.
///
/// Every process shares it, and it exists whether or not any terminal does.
/// A record carrying it was written by a version that asked the alias for its
/// own name and got the alias back, so it witnesses nothing ([ADR-0110]). No
/// terminal number maps to this spelling, so no launch can ever claim such a
/// directory again — which is what makes it garbage rather than an open
/// question.
///
/// [ADR-0110]: ../../docs/decisions/ADR-0110-record-the-terminal-witness-at-launch.md
pub(crate) const ALIAS: &str = "/dev/tty";

/// Why one session directory got the verdict it did.
///
/// The ground is the fact and [`Verdict`] is its projection, which is what
/// keeps a report able to say why without a renderer re-deriving it, and stops
/// a new ground from being added without stating what it means.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Ground {
    /// No record could be read for this directory.
    Unrecorded,
    /// This run cannot name the namespace the record is scoped by.
    Unplaced,
    /// The record's namespace is not this run's.
    Foreign,
    /// The record names the alias every process shares, so it names no pane.
    Alias,
    /// The recorded device exists.
    DevicePresent,
    /// The recorded device is gone.
    DeviceAbsent,
    /// Whether the recorded device exists could not be observed.
    DeviceUnobservable,
    /// The recorded leader is running, started when the record says.
    LeaderRunning,
    /// The recorded leader is absent, or its id now names another process.
    LeaderGone,
    /// The record belongs to a boot that has ended.
    LeaderForeignBoot,
    /// The recorded leader, or the boot to read it against, is unresolvable.
    LeaderUnreadable,
}

impl Ground {
    /// Projects this ground onto the verdict it establishes.
    pub(crate) const fn verdict(self) -> Verdict {
        match self {
            Self::DevicePresent | Self::LeaderRunning => Verdict::Live,
            Self::DeviceAbsent | Self::LeaderGone | Self::LeaderForeignBoot => Verdict::Dead,
            Self::Alias => Verdict::Orphaned,
            Self::Unrecorded
            | Self::Unplaced
            | Self::Foreign
            | Self::DeviceUnobservable
            | Self::LeaderUnreadable => Verdict::Unknown,
        }
    }

    /// Returns the machine spelling a document uses for this ground.
    pub(crate) const fn spelling(self) -> &'static str {
        match self {
            Self::Unrecorded => "unrecorded",
            Self::Unplaced => "unplaced",
            Self::Foreign => "foreign",
            Self::Alias => "alias",
            Self::DevicePresent => "device-present",
            Self::DeviceAbsent => "device-absent",
            Self::DeviceUnobservable => "device-unobservable",
            Self::LeaderRunning => "leader-running",
            Self::LeaderGone => "leader-gone",
            Self::LeaderForeignBoot => "leader-foreign-boot",
            Self::LeaderUnreadable => "leader-unreadable",
        }
    }
}

/// One session directory's liveness.
///
/// The session tree is the wrapper's own, so the only question worth asking of
/// a directory in it is whether this run can prove it is still in use. Exactly
/// one verdict says it can, and the rest are garbage; they stay separate words
/// because they differ in what a reader loses by collecting, which is the
/// sentence a report owes them ([ADR-0112]).
///
/// [ADR-0112]: ../../docs/decisions/ADR-0112-keep-only-the-session-proven-live.md
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Verdict {
    /// The recorded terminal still exists; the one verdict that is kept.
    Live,
    /// The recorded terminal existed and is provably gone.
    Dead,
    /// No terminal ever owned it, and none can ever claim it.
    Orphaned,
    /// This run cannot decide, which is a gap in a tree the wrapper owns
    /// rather than a reason to keep carrying it.
    Unknown,
}

impl Verdict {
    /// Returns the word a report uses for this verdict.
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Live => "live",
            Self::Dead => "dead",
            Self::Orphaned => "orphaned",
            Self::Unknown => "unknown",
        }
    }

    /// Reports whether `session clean` removes a directory judged this way.
    ///
    /// The single owner of the policy, so the survey, the collector, the
    /// prompt, and both renderers cannot disagree about what the verb takes.
    pub(crate) const fn collectable(self) -> bool {
        !matches!(self, Self::Live)
    }
}

/// Judges one marker against what this run observed, and says on what ground.
///
/// This answers one question — can this run prove the directory is still in
/// use — so every ground but the two live ones is a way of failing to prove it
/// ([ADR-0112]). Inside scope the tty rung is live while its device exists —
/// boot is ignored deliberately, because a reopened slot after reboot is the
/// same slot — and the leader rung is live while its process id and start time
/// match under the recorded boot, dead under a foreign boot, and undecidable
/// when either boot is unreadable.
///
/// A marker whose namespace component is not this run's stays its own ground:
/// the name it witnesses was issued somewhere this kernel cannot look, so the
/// report can say that rather than implying a terminal was checked and missed.
///
/// One device is judged before it is looked for. A record naming [`ALIAS`]
/// witnesses no pane, so it is neither live nor dead but orphaned: it is the
/// residue of a launch that could not tell terminals apart and gave every one
/// of them the same directory, and no name this version issues can ever land
/// on it again.
///
/// [ADR-0112]: ../../docs/decisions/ADR-0112-keep-only-the-session-proven-live.md
pub(crate) fn judge(marker: &Marker, observed: &Observed) -> Ground {
    let Some(current) = &observed.namespace else {
        return Ground::Unplaced;
    };
    if current != &marker.namespace {
        return Ground::Foreign;
    }
    match &marker.witness {
        Recorded::Tty { device } if device == ALIAS => Ground::Alias,
        Recorded::Tty { .. } => match observed.device_present {
            Some(true) => Ground::DevicePresent,
            Some(false) => Ground::DeviceAbsent,
            None => Ground::DeviceUnobservable,
        },
        Recorded::SessionLeader { started, .. } => match (&marker.boot, &observed.boot) {
            (Some(recorded), Some(current)) if recorded != current => Ground::LeaderForeignBoot,
            (Some(_), Some(_)) => match observed.leader {
                LeaderObservation::Running { started: now } if now == *started => {
                    Ground::LeaderRunning
                }
                // A recycled id is as gone as an absent one: the start time is
                // what tells this process from the one the record witnessed.
                LeaderObservation::Absent | LeaderObservation::Running { .. } => Ground::LeaderGone,
                LeaderObservation::Unreadable => Ground::LeaderUnreadable,
            },
            _ => Ground::LeaderUnreadable,
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

    /// Asserts the ground a judgment stands on and the verdict it projects.
    #[track_caller]
    fn judged(marker: &Marker, facts: &Observed, ground: Ground, verdict: Verdict) {
        assert_eq!(judge(marker, facts), ground);
        assert_eq!(ground.verdict(), verdict);
    }

    #[test]
    fn a_present_device_in_scope_is_live() {
        let marker = tty_marker();
        judged(
            &marker,
            &observed(&marker),
            Ground::DevicePresent,
            Verdict::Live,
        );
    }

    #[test]
    fn an_absent_device_in_scope_is_dead() {
        let marker = tty_marker();
        let facts = Observed {
            device_present: Some(false),
            ..observed(&marker)
        };
        judged(&marker, &facts, Ground::DeviceAbsent, Verdict::Dead);
    }

    #[test]
    fn an_unobservable_device_is_unknown() {
        let marker = tty_marker();
        let facts = Observed {
            device_present: None,
            ..observed(&marker)
        };
        judged(
            &marker,
            &facts,
            Ground::DeviceUnobservable,
            Verdict::Unknown,
        );
    }

    /// The tty rung ignores boot deliberately: a reopened slot after reboot is
    /// the same slot ([ADR-0112]).
    ///
    /// [ADR-0112]: ../../../docs/decisions/ADR-0112-keep-only-the-session-proven-live.md
    #[test]
    fn the_tty_rung_survives_a_reboot() {
        let marker = tty_marker();
        let facts = Observed {
            boot: Some("another-boot".to_owned()),
            ..observed(&marker)
        };
        judged(&marker, &facts, Ground::DevicePresent, Verdict::Live);
    }

    #[test]
    fn a_foreign_namespace_is_unknown_not_dead() {
        for marker in [tty_marker(), leader_marker()] {
            let facts = Observed {
                namespace: Some("mnt-000000000000000000000000".parse().expect("identifier")),
                ..observed(&marker)
            };
            judged(&marker, &facts, Ground::Foreign, Verdict::Unknown);
        }
    }

    #[test]
    fn an_unreadable_namespace_is_unknown() {
        for marker in [tty_marker(), leader_marker()] {
            let facts = Observed {
                namespace: None,
                ..observed(&marker)
            };
            judged(&marker, &facts, Ground::Unplaced, Verdict::Unknown);
        }
    }

    #[test]
    fn a_matching_leader_in_scope_is_live() {
        let marker = leader_marker();
        judged(
            &marker,
            &observed(&marker),
            Ground::LeaderRunning,
            Verdict::Live,
        );
    }

    #[test]
    fn an_absent_leader_is_dead() {
        let marker = leader_marker();
        let facts = Observed {
            leader: LeaderObservation::Absent,
            ..observed(&marker)
        };
        judged(&marker, &facts, Ground::LeaderGone, Verdict::Dead);
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
        judged(&marker, &facts, Ground::LeaderGone, Verdict::Dead);
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
        judged(&marker, &facts, Ground::LeaderForeignBoot, Verdict::Dead);
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
        judged(
            &marker,
            &unreadable,
            Ground::LeaderUnreadable,
            Verdict::Unknown,
        );
        let bootless = Observed {
            boot: None,
            ..observed(&marker)
        };
        judged(
            &marker,
            &bootless,
            Ground::LeaderUnreadable,
            Verdict::Unknown,
        );
        let unrecorded = Marker {
            boot: None,
            ..leader_marker()
        };
        judged(
            &unrecorded,
            &observed(&unrecorded),
            Ground::LeaderUnreadable,
            Verdict::Unknown,
        );
    }

    /// The residue of the defect that named every pane after the alias every
    /// process shares. The record witnesses no terminal and no name this
    /// version issues can land on its directory again, so it is orphaned
    /// rather than merely undecidable — and the device it names exists, which
    /// is exactly why the alias is judged before it is looked for.
    #[test]
    fn a_recorded_alias_witnesses_no_pane() {
        let marker = Marker {
            witness: Recorded::Tty {
                device: ALIAS.to_owned(),
            },
            ..tty_marker()
        };
        let facts = Observed {
            device_present: Some(true),
            ..observed(&marker)
        };
        judged(&marker, &facts, Ground::Alias, Verdict::Orphaned);
    }

    /// The whole collection policy in one assertion: a directory is kept only
    /// while this run can prove its terminal is still there, and every other
    /// verdict is garbage ([ADR-0112]).
    ///
    /// [ADR-0112]: ../../../docs/decisions/ADR-0112-keep-only-the-session-proven-live.md
    #[test]
    fn every_verdict_but_live_is_collectable() {
        assert!(!Verdict::Live.collectable());
        for verdict in [Verdict::Dead, Verdict::Orphaned, Verdict::Unknown] {
            assert!(verdict.collectable(), "{verdict:?}");
        }
    }

    /// Every verdict spells itself, and no two spell the same, so a document
    /// naming one cannot leave a reader guessing which it meant.
    #[test]
    fn every_verdict_carries_its_own_spelling() {
        let verdicts = [
            Verdict::Live,
            Verdict::Dead,
            Verdict::Orphaned,
            Verdict::Unknown,
        ];
        let mut spellings: Vec<&str> = verdicts.iter().map(|it| it.as_str()).collect();
        spellings.sort_unstable();
        spellings.dedup();
        assert_eq!(spellings.len(), verdicts.len());
    }

    /// Every ground spells itself, and no two spell the same, which is what
    /// lets a document name one without a reader guessing which it meant.
    #[test]
    fn every_ground_carries_its_own_spelling() {
        let grounds = [
            Ground::Unrecorded,
            Ground::Unplaced,
            Ground::Foreign,
            Ground::Alias,
            Ground::DevicePresent,
            Ground::DeviceAbsent,
            Ground::DeviceUnobservable,
            Ground::LeaderRunning,
            Ground::LeaderGone,
            Ground::LeaderForeignBoot,
            Ground::LeaderUnreadable,
        ];
        let mut spellings: Vec<&str> = grounds.iter().map(|it| it.spelling()).collect();
        spellings.sort_unstable();
        spellings.dedup();
        assert_eq!(spellings.len(), grounds.len());
    }
}
