//! Parses the checked-in child inventory fixture. Not a client of the child:
//! this module never spawns a process and never touches the network, which is
//! what makes the audit reproducible on a machine with no `claude` installed.

use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;

use crate::audit::{FIXTURE, Violation};

/// Spellings the fixture must carry for the audit to mean anything. A fixture
/// that parsed to something empty would make every collision check pass
/// vacuously, so the sentinels are checked before the comparison runs.
const FLAG_SENTINELS: &[&str] = &["--help", "--settings", "--verbose", "--version", "-h", "-v"];
const VERB_SENTINELS: &[&str] = &["auth", "doctor"];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    child_version: String,
    measured_on: String,
    source_command: String,
    // Deserialized as sequences rather than sets: a set collapses a duplicate
    // silently, and a duplicate is the visible symptom of a bad merge.
    top_level_flags: Vec<String>,
    top_level_verbs: Vec<String>,
    aliases: BTreeMap<String, String>,
    subcommand_flags: BTreeMap<String, Vec<String>>,
}

#[derive(Debug)]
pub(crate) struct Inventory {
    pub(crate) child_version: String,
    pub(crate) measured_on: String,
    pub(crate) flags: BTreeSet<String>,
    pub(crate) verbs: BTreeSet<String>,
    pub(crate) aliases: BTreeMap<String, String>,
    pub(crate) subcommand_flags: BTreeMap<String, BTreeSet<String>>,
}

pub(crate) fn parse(yaml: &str) -> Result<Inventory, Vec<Violation>> {
    let raw: Raw = match serde_yaml_ng::from_str(yaml) {
        Ok(raw) => raw,
        Err(err) => {
            let message = format!("the fixture does not parse: {err}");
            return Err(vec![match err.location() {
                Some(at) => Violation::at(FIXTURE, at.line(), message),
                None => Violation::whole(FIXTURE, message),
            }]);
        }
    };

    let mut found = Vec::new();

    if !is_version(&raw.child_version) {
        found.push(Violation::whole(
            FIXTURE,
            format!(
                "child_version `{}` is not a three-part version",
                raw.child_version
            ),
        ));
    }
    if !is_date(&raw.measured_on) {
        found.push(Violation::whole(
            FIXTURE,
            format!(
                "measured_on `{}` is not an ISO calendar date",
                raw.measured_on
            ),
        ));
    }
    if raw.source_command.trim().is_empty() {
        found.push(Violation::whole(
            FIXTURE,
            "source_command is empty; a fixture that does not say how it was \
            measured cannot be re-measured",
        ));
    }

    check_list("top_level_flags", &raw.top_level_flags, true, &mut found);
    check_list("top_level_verbs", &raw.top_level_verbs, false, &mut found);
    for (verb, flags) in &raw.subcommand_flags {
        check_list(&format!("subcommand_flags.{verb}"), flags, true, &mut found);
    }

    let flags: BTreeSet<String> = raw.top_level_flags.iter().cloned().collect();
    let verbs: BTreeSet<String> = raw.top_level_verbs.iter().cloned().collect();

    for sentinel in FLAG_SENTINELS {
        if !flags.contains(*sentinel) {
            found.push(Violation::whole(
                FIXTURE,
                format!(
                    "top_level_flags does not contain `{sentinel}`; the fixture is \
                    empty or truncated and every collision check would pass \
                    vacuously"
                ),
            ));
        }
    }
    for sentinel in VERB_SENTINELS {
        if !verbs.contains(*sentinel) {
            found.push(Violation::whole(
                FIXTURE,
                format!(
                    "top_level_verbs does not contain `{sentinel}`; the fixture is \
                    empty or truncated and every overlap check would pass \
                    vacuously"
                ),
            ));
        }
    }

    for (alias, meaning) in &raw.aliases {
        for (label, spelling) in [("key", alias), ("value", meaning)] {
            if !flags.contains(spelling) {
                found.push(Violation::whole(
                    FIXTURE,
                    format!("aliases {label} `{spelling}` is not in top_level_flags"),
                ));
            }
        }
    }
    for verb in raw.subcommand_flags.keys() {
        if !verbs.contains(verb) {
            found.push(Violation::whole(
                FIXTURE,
                format!("subcommand_flags key `{verb}` is not in top_level_verbs"),
            ));
        }
    }

    if !found.is_empty() {
        return Err(found);
    }

    Ok(Inventory {
        child_version: raw.child_version,
        measured_on: raw.measured_on,
        flags,
        verbs,
        aliases: raw.aliases,
        subcommand_flags: raw
            .subcommand_flags
            .into_iter()
            .map(|(verb, flags)| (verb, flags.into_iter().collect()))
            .collect(),
    })
}

/// Strict ascending order carries uniqueness with it, so one pass proves both.
fn check_list(name: &str, entries: &[String], flags: bool, found: &mut Vec<Violation>) {
    if entries.is_empty() {
        found.push(Violation::whole(FIXTURE, format!("{name} is empty")));
        return;
    }
    for pair in entries.windows(2) {
        if pair[0] >= pair[1] {
            found.push(Violation::whole(
                FIXTURE,
                format!(
                    "{name} is not byte-sorted and unique: `{}` precedes `{}`",
                    pair[0], pair[1]
                ),
            ));
        }
    }
    for entry in entries {
        if entry.is_empty() {
            found.push(Violation::whole(
                FIXTURE,
                format!("{name} has an empty entry"),
            ));
        } else if flags != entry.starts_with('-') {
            let expected = if flags {
                "a flag spelling starting with `-`"
            } else {
                "a verb name"
            };
            found.push(Violation::whole(
                FIXTURE,
                format!("{name} entry `{entry}` is not {expected}"),
            ));
        }
    }
}

fn is_version(value: &str) -> bool {
    let parts: Vec<&str> = value.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
}

fn is_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && [0, 1, 2, 3, 5, 6, 8, 9]
            .iter()
            .all(|&i| bytes[i].is_ascii_digit())
}
