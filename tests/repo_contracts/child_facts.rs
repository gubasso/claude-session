//! ADR-0089 permits a child-owned name in this repository only against a named
//! wrapper obligation. That is a fact about the whole tree — an identifier and
//! the registry entry that justifies it live in different files — so no lint
//! can express it.
//!
//! The gate is deliberately mechanical and complete for identifiers. Child
//! behaviour carried as prose is governed by the same obligation test and
//! enforced at review, which the registry's own header states rather than
//! leaving a reader to assume this gate covers it.

use std::collections::BTreeSet;

use serde::Deserialize;

use crate::violation::{Violation, assert_clean};
use crate::{read, tree};

const REGISTRY: &str = "docs/reference/child-facts.yaml";
const SLICES: &str = "docs/plan/slices/";

/// The registry names every fact by construction, and this file names them in
/// its sentinels and its doctored literals. Both would otherwise have to
/// register themselves as carry sites, which would make each entry its own
/// justification. Neither is skipped silently: the sentinels below assert the
/// scan still reaches the files that matter.
const SELF_REFERENTIAL: &[&str] = &[REGISTRY, "tests/repo_contracts/child_facts.rs"];

/// The prefixes that begin a child-owned identifier. `CLAUDE_SESSION_` is
/// absent on purpose: it is the wrapper's own namespace, and scanning it would
/// demand a registry entry for every internal variable.
const DISCOVERY: &[&str] = &["ANTHROPIC_", "CLAUDE_CODE_", "CLAUDE_CONFIG_"];

/// Child-owned names that no prefix reaches. The prefixes above describe
/// environment variables, and a key inside the child's own configuration file
/// is spelled like anything else — so it is discoverable only by being named
/// here. The list is explicit rather than a pattern for that reason: a general
/// identifier search over this tree would demand a registry entry for every
/// word in it.
const LITERALS: &[&str] = &[
    "hasCompletedOnboarding",
    "hasTrustDialogAccepted",
    "hasCompletedProjectOnboarding",
    // No `DISCOVERY` prefix reaches this one: the wrapper's namespace scan
    // covers `ANTHROPIC_`, `CLAUDE_CODE_`, and `CLAUDE_CONFIG_`, and the
    // child's credential-store variable is in none of them.
    "CLAUDE_SECURESTORAGE_CONFIG_DIR",
    // A key inside the child's own peer registration, which the report reads
    // to name a session as its reader does (ADR-0114). Its companion in that
    // record is `name`, an ordinary English word this scan cannot reach.
    "procStart",
    // The child's own recommendation gate, which a launch answers in the
    // session directory it created (ADR-0117). Distinctive enough for the scan
    // to reach, unlike the two plugin-state filenames it is written beside.
    "lspRecommendationDisabled",
];

/// The obligations ADR-0089 names, as amended by ADR-0114, plus the temporary
/// state a contested carry sits in while a question decides it.
const OBLIGATIONS: &[&str] = &[
    "launch",
    "no-collision",
    "scope-our-claim",
    "name-the-subject",
    "contested",
];

/// Text this repository authors. A binary or generated file carries no carry
/// decision, and `tree` already excludes the build directories.
const SUFFIXES: &[&str] = &[
    ".jsonc", ".json", ".md", ".nix", ".rs", ".toml", ".yaml", ".yml",
];

/// Floors and sentinels, because a scan that silently matches nothing reports a
/// clean tree. There are seven entries and thirty-eight occurrences today. The
/// floors sit below that on purpose: removing an unearned carry is the work this
/// gate exists to support, so it must not have to be relaxed to do it.
const ENTRY_FLOOR: usize = 5;
const OCCURRENCE_FLOOR: usize = 25;
const FACT_SENTINELS: &[&str] = &["CLAUDE_CONFIG_DIR", "CLAUDE_CODE_OAUTH_TOKEN"];
const SITE_SENTINELS: &[&str] = &["src/services/child.rs", "docs/reference/accounts.md"];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Registry {
    carried: Vec<Entry>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    fact: String,
    obligation: String,
    why: String,
    sites: Vec<String>,
}

/// One identifier at one path. The pair is the unit the registry justifies, so
/// a name that is registered but appears somewhere new is still a finding.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Occurrence {
    fact: String,
    path: String,
}

fn registry() -> (Registry, Vec<Violation>) {
    let text = read(REGISTRY);
    match serde_yaml_ng::from_str::<Registry>(&text) {
        Ok(parsed) => (parsed, Vec::new()),
        Err(err) => (
            Registry {
                carried: Vec::new(),
            },
            vec![Violation::whole(
                REGISTRY,
                format!("the registry does not parse: {err}"),
            )],
        ),
    }
}

/// True when `at` begins an identifier rather than continuing one, so a longer
/// name that merely contains a prefix is not reported under it.
fn starts_identifier(text: &str, at: usize) -> bool {
    text[..at]
        .chars()
        .next_back()
        .is_none_or(|previous| !previous.is_ascii_alphanumeric() && previous != '_')
}

/// The identifier beginning at `at`, in the child's screaming-snake spelling.
fn identifier(text: &str, at: usize) -> String {
    text[at..]
        .chars()
        .take_while(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || *c == '_')
        .collect()
}

/// True when nothing continues the identifier past `end`, so a literal is not
/// matched inside a longer name.
fn ends_identifier(text: &str, end: usize) -> bool {
    text[end..]
        .chars()
        .next()
        .is_none_or(|next| !next.is_ascii_alphanumeric() && next != '_')
}

fn scan(path: &str, text: &str) -> Vec<Occurrence> {
    let mut found = BTreeSet::new();
    for literal in LITERALS {
        for (at, _) in text.match_indices(literal) {
            if starts_identifier(text, at) && ends_identifier(text, at + literal.len()) {
                found.insert(Occurrence {
                    fact: (*literal).to_string(),
                    path: path.to_string(),
                });
            }
        }
    }
    for prefix in DISCOVERY {
        for (at, _) in text.match_indices(prefix) {
            if !starts_identifier(text, at) {
                continue;
            }
            let fact = identifier(text, at);
            // A bare prefix is prose about the namespace, not a name.
            if fact.len() > prefix.len() {
                found.insert(Occurrence {
                    fact,
                    path: path.to_string(),
                });
            }
        }
    }
    found.into_iter().collect()
}

/// Every occurrence in the tree, less the two self-referential files.
fn occurrences() -> Vec<Occurrence> {
    let mut found = Vec::new();
    for path in tree::files("", SUFFIXES) {
        if SELF_REFERENTIAL.contains(&path.as_str()) {
            continue;
        }
        found.extend(scan(&path, &read(&path)));
    }
    found.sort();
    found
}

fn unregistered(found: &[Occurrence], carried: &[Entry]) -> Vec<Violation> {
    found
        .iter()
        .filter(|occurrence| {
            !carried.iter().any(|entry| {
                entry.fact == occurrence.fact && entry.sites.contains(&occurrence.path)
            })
        })
        .map(|occurrence| {
            let known = carried.iter().any(|entry| entry.fact == occurrence.fact);
            let cause = if known {
                format!(
                    "{} is carried at a site the registry does not list",
                    occurrence.fact
                )
            } else {
                format!(
                    "{} is a child-owned name with no registry entry",
                    occurrence.fact
                )
            };
            let where_ = if occurrence.path.starts_with(SLICES) {
                format!("{cause}; a slice entry may not carry one")
            } else {
                cause
            };
            Violation::whole(occurrence.path.clone(), where_)
        })
        .collect()
}

/// An entry, or one of its sites, that matches nothing. Without this the
/// registry decays into an allowlist of names nobody carries, and the gate
/// above keeps passing while it does.
fn dead(found: &[Occurrence], carried: &[Entry]) -> Vec<Violation> {
    let mut violations = Vec::new();
    for entry in carried {
        if !found.iter().any(|occurrence| occurrence.fact == entry.fact) {
            violations.push(Violation::whole(
                REGISTRY,
                format!("{} is registered but appears nowhere", entry.fact),
            ));
            continue;
        }
        for site in &entry.sites {
            if !found
                .iter()
                .any(|occurrence| occurrence.fact == entry.fact && &occurrence.path == site)
            {
                violations.push(Violation::whole(
                    REGISTRY,
                    format!("{} lists {site}, which does not carry it", entry.fact),
                ));
            }
        }
    }
    violations
}

fn well_formed(carried: &[Entry]) -> Vec<Violation> {
    let mut violations = Vec::new();
    let mut seen: Vec<&str> = Vec::new();
    for entry in carried {
        if seen.contains(&entry.fact.as_str()) {
            violations.push(Violation::whole(
                REGISTRY,
                format!("{} has more than one entry", entry.fact),
            ));
        }
        seen.push(&entry.fact);

        if !OBLIGATIONS.contains(&entry.obligation.as_str()) {
            violations.push(Violation::whole(
                REGISTRY,
                format!(
                    "{} names obligation {}, which ADR-0089 does not",
                    entry.fact, entry.obligation
                ),
            ));
        }
        // A contested carry is only temporary while something is deciding it,
        // so the reason has to name the question that will end it.
        if entry.obligation == "contested" && !entry.why.contains("Q-") {
            violations.push(Violation::whole(
                REGISTRY,
                format!("{} is contested but names no open question", entry.fact),
            ));
        }
        if entry.why.trim().is_empty() || entry.sites.is_empty() {
            violations.push(Violation::whole(
                REGISTRY,
                format!("{} needs both a reason and at least one site", entry.fact),
            ));
        }
    }
    violations
}

#[test]
fn every_carried_child_fact_names_its_obligation() {
    let (parsed, mut found) = registry();
    assert!(
        parsed.carried.len() >= ENTRY_FLOOR,
        "the registry parsed to {} entries, below the floor of {ENTRY_FLOOR}",
        parsed.carried.len()
    );
    for sentinel in FACT_SENTINELS {
        assert!(
            parsed.carried.iter().any(|entry| entry.fact == *sentinel),
            "{sentinel} is missing from the registry"
        );
    }
    found.extend(well_formed(&parsed.carried));
    assert_clean(&found);
}

#[test]
fn a_child_owned_name_appears_only_at_a_registered_site() {
    let found = occurrences();
    assert!(
        found.len() >= OCCURRENCE_FLOOR,
        "the scan found {} occurrences, below the floor of {OCCURRENCE_FLOOR}",
        found.len()
    );
    for sentinel in SITE_SENTINELS {
        assert!(
            found.iter().any(|occurrence| occurrence.path == *sentinel),
            "{sentinel} carries a child-owned name and the scan missed it"
        );
    }
    let (parsed, mut violations) = registry();
    violations.extend(unregistered(&found, &parsed.carried));
    assert_clean(&violations);
}

#[test]
fn a_registry_entry_matches_something_in_the_tree() {
    let (parsed, mut violations) = registry();
    violations.extend(dead(&occurrences(), &parsed.carried));
    assert_clean(&violations);
}

#[test]
fn a_slice_entry_carries_no_unregistered_child_owned_name() {
    let (parsed, mut violations) = registry();
    let entries: Vec<Occurrence> = occurrences()
        .into_iter()
        .filter(|occurrence| occurrence.path.starts_with(SLICES))
        .collect();
    violations.extend(unregistered(&entries, &parsed.carried));
    assert_clean(&violations);
}

/// The gate proves it can go red. A doctored occurrence and a doctored entry
/// exercise both directions, because a scan that cannot fail and a registry
/// that cannot rot are the two ways this gate would quietly stop working.
#[test]
fn the_gate_reports_a_doctored_carry() {
    let entry = Entry {
        fact: "ANTHROPIC_EXAMPLE".to_string(),
        obligation: "launch".to_string(),
        why: "doctored".to_string(),
        sites: vec!["src/main.rs".to_string()],
    };

    let smuggled = Occurrence {
        fact: "ANTHROPIC_SMUGGLED".to_string(),
        path: format!("{SLICES}015-child-fact-delegation/README.md"),
    };
    let reported = unregistered(std::slice::from_ref(&smuggled), &[]);
    assert_eq!(reported.len(), 1, "an unregistered carry must be reported");
    assert!(
        reported[0]
            .to_string()
            .contains("a slice entry may not carry one"),
        "a slice entry names its own rule: {}",
        reported[0]
    );

    assert_eq!(
        dead(&[], std::slice::from_ref(&entry)).len(),
        1,
        "an entry matching nothing must be reported"
    );
    assert_eq!(
        dead(
            &[Occurrence {
                fact: entry.fact.clone(),
                path: "src/other.rs".to_string(),
            }],
            std::slice::from_ref(&entry),
        )
        .len(),
        1,
        "a site that does not carry the fact must be reported"
    );

    let mistyped = Entry {
        obligation: "because".to_string(),
        ..entry
    };
    assert_eq!(
        well_formed(std::slice::from_ref(&mistyped)).len(),
        1,
        "an obligation ADR-0089 does not name must be reported"
    );

    // The scanner itself: a longer name that merely contains a prefix is not a
    // match, and a bare prefix is prose rather than a name.
    assert!(scan("f.md", "XANTHROPIC_API_KEY").is_empty());
    assert!(scan("f.md", "the CLAUDE_CODE_ namespace").is_empty());
    assert_eq!(scan("f.md", "set ANTHROPIC_API_KEY now").len(), 1);
}
