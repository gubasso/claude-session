//! The child's own name for one running agent.
//!
//! The child registers every session it runs under the `sessions` name of its
//! configuration directory, which a launch points at the shared peer registry
//! ([ADR-0108]). One of those registrations holds the name a person sees in
//! their status line and sets with the child's own rename, and a report about
//! a session is unreadable without it: the wrapper's directory name is an
//! internal identifier, and the reader has never seen it ([ADR-0114]).
//!
//! This module reads no file — that is the session service — and carries only
//! the name and the pair that proves whose name it is. Everything else the
//! record holds belongs to the child ([ADR-0089]).
//!
//! [ADR-0089]: ../../docs/decisions/ADR-0089-carry-a-child-owned-fact-only-against-an-obligation.md
//! [ADR-0108]: ../../docs/decisions/ADR-0108-share-the-child-peer-registry-across-sessions.md
//! [ADR-0114]: ../../docs/decisions/ADR-0114-name-a-reported-session-as-the-child-does.md

/// The longest name a report carries.
///
/// A name past it is refused rather than cut, for the reason an over-long
/// agent identifier is: a cut name is one two sessions could share, and a row
/// naming the wrong session is worse than a row naming none.
const LONGEST: usize = 64;

/// One child registration, reduced to what a report may say.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Registration {
    pid: u32,
    started: u64,
    name: String,
}

impl Registration {
    /// Reads one registration document, or refuses it.
    ///
    /// `None` for anything this wrapper cannot use as a name: a document that
    /// is not an object, a missing or ill-typed field, or a name the checks
    /// below reject. The child owns the schema, so unknown fields are ignored
    /// rather than refused — a record that grows a field still names its
    /// session.
    pub(crate) fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let value: serde_json::Value = serde_json::from_slice(bytes).ok()?;
        let pid = u32::try_from(value.get("pid")?.as_u64()?).ok()?;
        let started = ticks(value.get("procStart")?)?;
        Self::new(pid, started, value.get("name")?.as_str()?)
    }

    /// Builds a registration from the three facts, validating the name.
    ///
    /// A control character is refused rather than escaped or stripped: the
    /// name is rendered into a report the wrapper decorates itself, and a
    /// stripped name is no longer the name the child answers to.
    pub(crate) fn new(pid: u32, started: u64, name: &str) -> Option<Self> {
        let refused = name.is_empty()
            || name.chars().count() > LONGEST
            || name.chars().any(char::is_control)
            || name.trim() != name;
        (!refused).then(|| Self {
            pid,
            started,
            name: name.to_owned(),
        })
    }

    /// Reports whether this registration is the one that agent wrote.
    ///
    /// Both halves, because a process identifier is reused within one boot:
    /// with the identifier alone, a registration a live agent wrote would
    /// lend its name to the dead session directory that ran under the same
    /// number ([ADR-0114]).
    ///
    /// [ADR-0114]: ../../docs/decisions/ADR-0114-name-a-reported-session-as-the-child-does.md
    pub(crate) const fn names(&self, pid: u32, started: u64) -> bool {
        self.pid == pid && self.started == started
    }

    /// Borrows the name the child registered.
    pub(crate) fn name(&self) -> &str {
        &self.name
    }
}

/// Reads a start time the child records as text or as a number.
///
/// The observed record spells it as a decimal string, which is a shape a
/// JavaScript writer picks to keep a large integer exact. Reading a number too
/// costs one branch and makes the join survive that choice changing.
fn ticks(value: &serde_json::Value) -> Option<u64> {
    match value {
        serde_json::Value::String(text) => text.parse().ok(),
        other => other.as_u64(),
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    fn document(name: &str) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({
            "pid": 2_675_277,
            "sessionId": "12fdd685-1e9b-416f-8b8d-4431f3fd5e05",
            "procStart": "183335578",
            "name": name,
            "nameSource": "derived",
        }))
        .expect("a document")
    }

    #[test]
    fn a_registration_carries_the_name_and_the_pair_that_proves_it() {
        let one = Registration::from_bytes(&document("claude-session-53")).expect("a name");
        assert_eq!(one.name(), "claude-session-53");
        assert!(one.names(2_675_277, 183_335_578));
    }

    /// The whole reason the start time is read: a registration a live agent
    /// wrote must not name the exited session that ran under its identifier.
    #[test]
    fn a_reused_identifier_does_not_carry_another_agents_name() {
        let one = Registration::from_bytes(&document("claude-session-53")).expect("a name");
        assert!(!one.names(2_675_277, 1));
        assert!(!one.names(1, 183_335_578));
    }

    #[test]
    fn a_start_time_is_read_as_text_or_as_a_number() {
        let text = Registration::from_bytes(
            &serde_json::to_vec(&serde_json::json!({"pid": 7, "procStart": "42", "name": "one"}))
                .expect("a document"),
        )
        .expect("a name");
        let number = Registration::from_bytes(
            &serde_json::to_vec(&serde_json::json!({"pid": 7, "procStart": 42, "name": "one"}))
                .expect("a document"),
        )
        .expect("a name");
        assert_eq!(text, number);
    }

    /// The report is text the wrapper lays out and decorates, so a name that
    /// could move a column or forge a status word names nothing.
    #[test]
    fn a_name_holding_a_control_character_is_refused() {
        for name in ["two\nlines", "a\tcolumn", "\u{1b}[31mred\u{1b}[0m"] {
            assert!(
                Registration::from_bytes(&document(name)).is_none(),
                "{name:?} should be refused"
            );
        }
    }

    #[test]
    fn an_empty_padded_or_over_long_name_is_refused() {
        for name in ["", " ", " padded", "padded ", &"n".repeat(LONGEST + 1)] {
            assert!(
                Registration::from_bytes(&document(name)).is_none(),
                "{name:?} should be refused"
            );
        }
        assert!(
            Registration::from_bytes(&document(&"n".repeat(LONGEST))).is_some(),
            "the longest name a report carries is carried"
        );
    }

    /// The child owns the schema, so a field this wrapper never reads may
    /// appear, change, or go without taking the name with it.
    #[test]
    fn an_unknown_field_does_not_refuse_the_record() {
        let grown = serde_json::to_vec(&serde_json::json!({
            "pid": 7,
            "procStart": "42",
            "name": "one",
            "somethingNewTheChildAdded": {"nested": true},
        }))
        .expect("a document");
        assert!(Registration::from_bytes(&grown).is_some());
    }

    #[test]
    fn a_document_missing_what_the_join_needs_names_nothing() {
        for value in [
            serde_json::json!({"procStart": "42", "name": "one"}),
            serde_json::json!({"pid": 7, "name": "one"}),
            serde_json::json!({"pid": 7, "procStart": "42"}),
            serde_json::json!({"pid": 7, "procStart": "ticks", "name": "one"}),
            serde_json::json!({"pid": -1, "procStart": "42", "name": "one"}),
            serde_json::json!({"pid": 7, "procStart": "42", "name": 5}),
            serde_json::json!(["not", "an", "object"]),
        ] {
            let bytes = serde_json::to_vec(&value).expect("a document");
            assert!(
                Registration::from_bytes(&bytes).is_none(),
                "{value} should name nothing"
            );
        }
    }
}
