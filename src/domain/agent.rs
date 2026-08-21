//! Naming the coding agent one session directory belongs to.
//!
//! A session is one running coding agent, and only that ([ADR-0113]). The
//! wrapper execs the child, so the process that will be the agent is this
//! process: its identifier and start time, read before the exec, name the
//! directory and answer the liveness question afterwards.
//!
//! This module performs no syscall — that is `adapters::host` — and decides
//! no path, which is `domain::paths`.
//!
//! [ADR-0113]: ../../docs/decisions/ADR-0113-key-a-session-to-its-running-agent.md

use std::str::FromStr as _;

use crate::domain::{identifier::Identifier, namespace::Namespace};

/// One run's agent: the path component, the namespace its process identifier
/// is unique inside, and the pair naming the process itself.
///
/// The start time is part of the identity rather than a field beside it,
/// because a process identifier alone is reused within one boot and would let
/// a later agent inherit an earlier one's state directory.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Agent {
    id: Identifier,
    namespace: Namespace,
    pid: u32,
    started: u64,
}

impl Agent {
    /// Names the agent this run is about to become, or refuses.
    ///
    /// `None` when the pair cannot spell an identifier, which needs a process
    /// identifier past every value Linux issues. Refusing beats truncating:
    /// a truncated name is one two agents could share.
    ///
    /// The namespace is the process namespace, which is what issued the
    /// identifier and the one `/proc` reports it against.
    pub(crate) fn new(namespace: Namespace, pid: u32, started: u64) -> Option<Self> {
        Identifier::from_str(&spell(pid, started))
            .ok()
            .map(|id| Self {
                id,
                namespace,
                pid,
                started,
            })
    }

    /// Borrows the validated path component.
    pub(crate) const fn id(&self) -> &Identifier {
        &self.id
    }

    /// Borrows the namespace this agent's identifier is unique inside.
    pub(crate) const fn namespace(&self) -> &Namespace {
        &self.namespace
    }

    /// Returns the process identifier the agent runs under.
    pub(crate) const fn pid(&self) -> u32 {
        self.pid
    }

    /// Returns the agent's start time in clock ticks since its kernel booted.
    pub(crate) const fn started(&self) -> u64 {
        self.started
    }
}

/// Spells the directory one agent's state lives in.
///
/// The process identifier stays decimal because it is the half a reader
/// recognises — it is what `ps` prints. The start time is hexadecimal because
/// it is a disambiguator nobody reads, and a decimal `u64` would spend twenty
/// of the identifier grammar's thirty-two bytes on it.
fn spell(pid: u32, started: u64) -> String {
    format!("agent-{pid}-{started:x}")
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::domain::namespace::{Discriminator, DiscriminatorSource, Kind, Namespace};
    use std::ffi::OsString;

    fn space(link: &str) -> Namespace {
        let machine = Discriminator::new(DiscriminatorSource::Machine, "machine-fixture")
            .expect("a value names a discriminator");
        Namespace::from_link(Kind::Pid, &OsString::from(link), &machine)
            .expect("a link names a namespace")
    }

    fn agent(pid: u32, started: u64) -> Option<Agent> {
        Agent::new(space("pid:[4026531836]"), pid, started)
    }

    #[test]
    fn an_agent_is_named_by_its_process_and_when_it_started() {
        let one = agent(1_965_996, 0x4f3_a2c1).expect("a process names an agent");
        assert_eq!(one.id().as_str(), "agent-1965996-4f3a2c1");
        assert_eq!(one.pid(), 1_965_996);
        assert_eq!(one.started(), 0x4f3_a2c1);
    }

    /// The whole reason the start time is in the name: an identifier the
    /// kernel reissued after the first agent exited must not walk into the
    /// first agent's directory.
    #[test]
    fn a_reused_process_identifier_does_not_inherit_the_earlier_directory() {
        let first = agent(4242, 1).expect("first");
        let second = agent(4242, 2).expect("second");
        assert_ne!(first.id(), second.id());
        assert_ne!(first, second);
    }

    /// Linux issues no identifier above `2^22`, so the longest name a running
    /// kernel can ask for still leaves room beside the namespace component.
    #[test]
    fn the_longest_name_a_kernel_can_ask_for_stays_an_identifier() {
        let longest = agent(4_194_304, u64::MAX).expect("the widest live pair");
        assert_eq!(longest.id().as_str(), "agent-4194304-ffffffffffffffff");
        assert_eq!(longest.id().as_str().len(), 30);
        assert_eq!(longest.namespace().id().as_str().len(), 28);
    }

    /// Two agents must never collapse onto one directory, so a pair that
    /// cannot spell an identifier is refused rather than cut. It takes a
    /// process identifier no kernel issues to reach this.
    #[test]
    fn an_impossible_process_identifier_is_refused_rather_than_truncated() {
        assert!(agent(u32::MAX, u64::MAX).is_none());
    }

    /// The namespace axis, for the same reason it existed before: a
    /// container's first agent and the host's can carry one process
    /// identifier, and must not carry one directory.
    #[test]
    fn one_process_identifier_in_two_namespaces_names_two_agents() {
        let host = Agent::new(space("pid:[4026531836]"), 1, 10).expect("host agent");
        let guest = Agent::new(space("pid:[4026533427]"), 1, 10).expect("container agent");
        assert_eq!(host.id(), guest.id());
        assert_ne!(host.namespace().id(), guest.namespace().id());
        assert_ne!(host, guest);
    }
}
