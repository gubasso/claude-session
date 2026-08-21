//! The record naming one session's agent, and the pure judgment over it.
//!
//! A session directory's name spells the agent that owns it, but a name is not
//! evidence: a directory can be moved, and a start time counts ticks from a
//! boot the name never states. This module owns the record a launch writes
//! beside the directory — the witness — and the judgment a later run takes
//! against it ([ADR-0110], [ADR-0113]).
//!
//! It performs no syscall: gathering the observations is the session service's,
//! and deciding the record's path is `domain::paths`. The judgment answers with
//! the ground it stands on, and the verdict is that ground's projection, so a
//! report can say why without re-deriving it.
//!
//! [ADR-0110]: ../../docs/decisions/ADR-0110-record-the-terminal-witness-at-launch.md
//! [ADR-0113]: ../../docs/decisions/ADR-0113-key-a-session-to-its-running-agent.md

use crate::domain::{agent::Agent, identifier::Identifier};

/// The record version this module writes, and the only one it reads.
///
/// Version `1` keyed a session to a terminal and is deliberately not read: it
/// witnesses a question this module no longer asks, so a directory carrying
/// one is unrecorded rather than reinterpreted ([ADR-0113]).
///
/// [ADR-0113]: ../../docs/decisions/ADR-0113-key-a-session-to-its-running-agent.md
const VERSION: u32 = 2;

/// The marker a launch writes beside the session directory it names.
///
/// `boot` is not optional. A start time is a count of ticks since some kernel
/// booted, so a record that cannot say which boot cannot be judged at all, and
/// writing one anyway would put an undecidable directory in a tree whose whole
/// premise is that every directory is decidable.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub(crate) struct Marker {
    version: u32,
    /// The process the agent runs as.
    pid: u32,
    /// That process's start time, in clock ticks since `boot`.
    started: u64,
    /// The namespace component the process identifier is unique inside.
    namespace: Identifier,
    /// The kernel boot the start time counts from.
    boot: String,
}

impl Marker {
    /// Builds the marker one launch records for its agent.
    ///
    /// `None` when this run cannot name the boot its own start time counts
    /// from; the caller logs the degradation and writes nothing.
    pub(crate) fn from_agent(agent: &Agent, boot: Option<String>) -> Option<Self> {
        Some(Self {
            version: VERSION,
            pid: agent.pid(),
            started: agent.started(),
            namespace: agent.namespace().id().clone(),
            boot: boot?,
        })
    }

    /// Returns the process the record names.
    pub(crate) const fn pid(&self) -> u32 {
        self.pid
    }

    /// Returns that process's recorded start time.
    pub(crate) const fn started(&self) -> u64 {
        self.started
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

/// What this run observed about a recorded agent process.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Process {
    /// No process carries the recorded identifier.
    Absent,
    /// A process carries the identifier, with this start time.
    Running { started: u64 },
    /// A process carries the identifier but its start time could not be read.
    Unreadable,
}

/// The facts a judgment needs, gathered by the caller for one marker.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Observed {
    /// This kernel's process-namespace component; `None` when underivable.
    pub(crate) namespace: Option<Identifier>,
    /// This kernel's boot identifier; `None` when unreadable.
    pub(crate) boot: Option<String>,
    /// What the recorded process identifier names now.
    pub(crate) process: Process,
}

/// Why one session directory got the verdict it did.
///
/// The ground is the fact and [`Verdict`] is its projection, which keeps a
/// report able to say why without a renderer re-deriving it, and stops a new
/// ground from being added without stating what it means.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Ground {
    /// No record this version reads could be read for this directory.
    Unrecorded,
    /// This run can name neither its namespace nor its boot, so it can judge
    /// nothing at all.
    Unplaced,
    /// The record's namespace is not this run's.
    Foreign,
    /// The record belongs to a boot that has ended.
    ForeignBoot,
    /// The recorded agent is running, started when the record says.
    Running,
    /// The recorded agent has exited, or its identifier now names another
    /// process.
    Gone,
    /// A process carries the identifier but its start time is unreadable.
    Unreadable,
}

impl Ground {
    /// Projects this ground onto the verdict it establishes.
    pub(crate) const fn verdict(self) -> Verdict {
        match self {
            Self::Running => Verdict::Live,
            Self::Gone | Self::ForeignBoot => Verdict::Dead,
            Self::Unrecorded | Self::Unplaced | Self::Foreign | Self::Unreadable => {
                Verdict::Unknown
            }
        }
    }

    /// Returns the machine spelling a document uses for this ground.
    pub(crate) const fn spelling(self) -> &'static str {
        match self {
            Self::Unrecorded => "unrecorded",
            Self::Unplaced => "unplaced",
            Self::Foreign => "foreign",
            Self::ForeignBoot => "foreign-boot",
            Self::Running => "running",
            Self::Gone => "gone",
            Self::Unreadable => "unreadable",
        }
    }
}

/// One session directory's liveness.
///
/// The session tree is the wrapper's own and a session is one running agent,
/// so the only question worth asking of a directory in it is whether that
/// agent is still running. Exactly one verdict says it is; the rest are
/// garbage, and stay separate words because they differ in what a reader
/// loses by collecting ([ADR-0112]).
///
/// [ADR-0112]: ../../docs/decisions/ADR-0112-keep-only-the-session-proven-live.md
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Verdict {
    /// The recorded agent is still running; the one verdict that is kept.
    Live,
    /// The recorded agent has exited.
    Dead,
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
/// One question, asked once: is the recorded agent still running. A boot that
/// has ended answers it without looking at any process, because none outlives
/// its kernel. Inside this boot the answer is the process identifier and the
/// start time together — a recycled identifier is as gone as an absent one,
/// and the start time is what tells the two apart.
///
/// A run that can name neither its namespace nor its boot is `Unplaced` and
/// judges nothing: it would read every directory as unaccounted for, and the
/// collector refuses on that ground rather than emptying a tree it cannot see.
/// A record whose namespace is not this run's stays its own ground, so a
/// report can say the name was issued somewhere this kernel cannot look rather
/// than implying a process was checked and missed.
pub(crate) fn judge(marker: &Marker, observed: &Observed) -> Ground {
    let (Some(namespace), Some(boot)) = (&observed.namespace, &observed.boot) else {
        return Ground::Unplaced;
    };
    if namespace != &marker.namespace {
        return Ground::Foreign;
    }
    if boot != &marker.boot {
        return Ground::ForeignBoot;
    }
    match observed.process {
        Process::Running { started } if started == marker.started => Ground::Running,
        Process::Absent | Process::Running { .. } => Ground::Gone,
        Process::Unreadable => Ground::Unreadable,
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::domain::namespace::{Discriminator, DiscriminatorSource, Kind, Namespace};
    use std::ffi::OsString;

    fn space() -> Namespace {
        let machine = Discriminator::new(DiscriminatorSource::Machine, "machine-fixture")
            .expect("a value names a discriminator");
        Namespace::from_link(Kind::Pid, &OsString::from("pid:[4026531836]"), &machine)
            .expect("a link names a namespace")
    }

    fn marker() -> Marker {
        let agent = Agent::new(space(), 4242, 987_654).expect("a process names an agent");
        Marker::from_agent(&agent, Some("boot-fixture".to_owned())).expect("a boot records")
    }

    fn observed(marker: &Marker) -> Observed {
        Observed {
            namespace: Some(marker.namespace().clone()),
            boot: Some("boot-fixture".to_owned()),
            process: Process::Running { started: 987_654 },
        }
    }

    /// Asserts the ground a judgment stands on and the verdict it projects.
    #[track_caller]
    fn judged(marker: &Marker, facts: &Observed, ground: Ground, verdict: Verdict) {
        assert_eq!(judge(marker, facts), ground);
        assert_eq!(ground.verdict(), verdict);
    }

    #[test]
    fn a_marker_roundtrips_through_its_document() {
        let marker = marker();
        let bytes = marker.to_bytes().expect("a marker serializes");
        assert_eq!(Marker::from_bytes(&bytes), Some(marker));
    }

    /// The predecessor's record answers a question this module stopped
    /// asking, so it is not read at all rather than reinterpreted.
    #[test]
    fn a_terminal_keyed_record_is_not_read() {
        let v1 = br#"{"version":1,"rung":"tty","device":"/dev/pts/3","namespace":"pid-1"}"#;
        assert_eq!(Marker::from_bytes(v1), None);
    }

    #[test]
    fn garbage_is_not_a_marker() {
        assert_eq!(Marker::from_bytes(b"not json"), None);
        assert_eq!(Marker::from_bytes(b"{}"), None);
    }

    /// A start time counts ticks from a boot, so a record that cannot name
    /// one is not written at all.
    #[test]
    fn a_record_without_a_boot_is_not_built() {
        let agent = Agent::new(space(), 1, 1).expect("a process names an agent");
        assert_eq!(Marker::from_agent(&agent, None), None);
    }

    #[test]
    fn a_running_agent_in_scope_is_live() {
        let marker = marker();
        judged(&marker, &observed(&marker), Ground::Running, Verdict::Live);
    }

    #[test]
    fn an_exited_agent_is_dead() {
        let marker = marker();
        let facts = Observed {
            process: Process::Absent,
            ..observed(&marker)
        };
        judged(&marker, &facts, Ground::Gone, Verdict::Dead);
    }

    /// The start time is what stops a reissued process identifier from
    /// keeping an earlier agent's directory alive.
    #[test]
    fn a_reissued_identifier_is_dead() {
        let marker = marker();
        let facts = Observed {
            process: Process::Running { started: 1 },
            ..observed(&marker)
        };
        judged(&marker, &facts, Ground::Gone, Verdict::Dead);
    }

    /// No process outlives its kernel, so a boot that has ended needs no
    /// process check at all.
    #[test]
    fn an_agent_under_a_foreign_boot_is_dead() {
        let marker = marker();
        let facts = Observed {
            boot: Some("another-boot".to_owned()),
            process: Process::Running { started: 987_654 },
            ..observed(&marker)
        };
        judged(&marker, &facts, Ground::ForeignBoot, Verdict::Dead);
    }

    #[test]
    fn a_foreign_namespace_is_unknown_rather_than_dead() {
        let marker = marker();
        let facts = Observed {
            namespace: Some("pid-000000000000000000000000".parse().expect("identifier")),
            ..observed(&marker)
        };
        judged(&marker, &facts, Ground::Foreign, Verdict::Unknown);
    }

    /// A run that cannot place itself judges nothing, whichever half it is
    /// missing; the collector refuses on this ground.
    #[test]
    fn a_run_that_cannot_place_itself_judges_nothing() {
        let marker = marker();
        for facts in [
            Observed {
                namespace: None,
                ..observed(&marker)
            },
            Observed {
                boot: None,
                ..observed(&marker)
            },
        ] {
            judged(&marker, &facts, Ground::Unplaced, Verdict::Unknown);
        }
    }

    #[test]
    fn an_unreadable_start_time_leaves_the_question_open() {
        let marker = marker();
        let facts = Observed {
            process: Process::Unreadable,
            ..observed(&marker)
        };
        judged(&marker, &facts, Ground::Unreadable, Verdict::Unknown);
    }

    /// The whole collection policy in one assertion: a directory is kept only
    /// while its agent is proven running, and every other verdict is garbage.
    #[test]
    fn every_verdict_but_live_is_collectable() {
        assert!(!Verdict::Live.collectable());
        for verdict in [Verdict::Dead, Verdict::Unknown] {
            assert!(verdict.collectable(), "{verdict:?}");
        }
    }

    /// Every ground and every verdict spells itself, and no two spell the
    /// same, which is what lets a document name one without a reader guessing.
    #[test]
    fn every_ground_and_verdict_carries_its_own_spelling() {
        let grounds = [
            Ground::Unrecorded,
            Ground::Unplaced,
            Ground::Foreign,
            Ground::ForeignBoot,
            Ground::Running,
            Ground::Gone,
            Ground::Unreadable,
        ];
        let mut spellings: Vec<&str> = grounds.iter().map(|it| it.spelling()).collect();
        spellings.sort_unstable();
        spellings.dedup();
        assert_eq!(spellings.len(), grounds.len());

        let verdicts = [Verdict::Live, Verdict::Dead, Verdict::Unknown];
        let mut words: Vec<&str> = verdicts.iter().map(|it| it.as_str()).collect();
        words.sort_unstable();
        words.dedup();
        assert_eq!(words.len(), verdicts.len());
    }
}
