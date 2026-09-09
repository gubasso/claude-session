//! The child's own name for one running agent.
//!
//! The child registers every session it runs under the `sessions` name of its
//! configuration directory, which a launch points at the shared peer registry
//! ([ADR-0108]). One of those registrations holds the name a person sees in
//! their status line and sets with the child's own rename, and a report about
//! a session is unreadable without it: the wrapper's directory name is an
//! internal identifier, and the reader has never seen it ([ADR-0114]).
//!
//! This module reads no file — that is the session service — and carries the
//! name, the pair that proves whose name it is, and the two descriptive facts
//! ADR-0124 permits. Everything else belongs to the child ([ADR-0089]).
//!
//! [ADR-0089]: ../../docs/decisions/ADR-0089-carry-a-child-owned-fact-only-against-an-obligation.md
//! [ADR-0108]: ../../docs/decisions/ADR-0108-share-the-child-peer-registry-across-sessions.md
//! [ADR-0114]: ../../docs/decisions/ADR-0114-name-a-reported-session-as-the-child-does.md
//! [ADR-0124]: ../../docs/decisions/ADR-0124-describe-a-reported-session-with-the-children-facts.md

use std::str::FromStr as _;

use crate::error::DomainError;

/// The longest name a report carries.
///
/// A name past it is refused rather than cut, for the reason an over-long
/// agent identifier is: a cut name is one two sessions could share, and a row
/// naming the wrong session is worse than a row naming none.
const LONGEST: usize = 64;
const LONGEST_DIRECTORY: usize = 4096;

/// The one subject a report can carry and a filter can name.
///
/// A subject is refused rather than cut because a cut name is one two sessions
/// could share, and naming the wrong session is worse than naming none.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Subject(String);

impl std::str::FromStr for Subject {
    type Err = DomainError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let refused = value.is_empty()
            || value.chars().count() > LONGEST
            || value.chars().any(char::is_control)
            || value.trim() != value;
        if refused {
            return Err(DomainError::InvalidArguments(
                "a session name must be 1 to 64 printable, unpadded characters".to_owned(),
            ));
        }
        Ok(Self(value.to_owned()))
    }
}

impl Subject {
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

/// One child registration, reduced to what a report may say.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Registration {
    pid: u32,
    started: u64,
    name: String,
    working_directory: Option<String>,
    child_status: Option<String>,
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
        let mut registration = Self::new(pid, started, value.get("name")?.as_str()?)?;
        registration.working_directory = value
            .get("cwd")
            .and_then(serde_json::Value::as_str)
            .filter(|value| {
                !value.is_empty()
                    && value.chars().count() <= LONGEST_DIRECTORY
                    && !value.chars().any(char::is_control)
            })
            .map(str::to_owned);
        registration.child_status = value
            .get("status")
            .and_then(serde_json::Value::as_str)
            .and_then(|value| Subject::from_str(value).ok())
            .map(|value| value.0);
        Some(registration)
    }

    /// Builds a registration from the three facts, validating the name.
    ///
    /// A control character is refused rather than escaped or stripped: the
    /// name is rendered into a report the wrapper decorates itself, and a
    /// stripped name is no longer the name the child answers to.
    pub(crate) fn new(pid: u32, started: u64, name: &str) -> Option<Self> {
        let name = Subject::from_str(name).ok()?;
        Some(Self {
            pid,
            started,
            name: name.0,
            working_directory: None,
            child_status: None,
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

    pub(crate) fn working_directory(&self) -> Option<&str> {
        self.working_directory.as_deref()
    }

    pub(crate) fn child_status(&self) -> Option<&str> {
        self.child_status.as_deref()
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

    #[test]
    fn a_subject_and_a_registered_name_share_one_alphabet() {
        for name in [
            "",
            " ",
            " padded",
            "padded ",
            "two\nlines",
            &"n".repeat(LONGEST + 1),
        ] {
            assert_eq!(
                name.parse::<Subject>().is_ok(),
                Registration::new(7, 42, name).is_some(),
                "{name:?}"
            );
        }
    }

    #[test]
    fn a_working_directory_is_not_held_to_the_name_rule() {
        let long = format!(" /{} ", "d".repeat(LONGEST + 1));
        let value = serde_json::json!({
            "pid": 7,
            "procStart": "42",
            "name": "one",
            "cwd": long,
            "status": "busy",
        });
        let registration =
            Registration::from_bytes(&serde_json::to_vec(&value).expect("registration document"))
                .expect("registration");
        assert_eq!(registration.working_directory(), value["cwd"].as_str());
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
