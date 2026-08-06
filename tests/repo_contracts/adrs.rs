//! ADR-0076 and ADR-0078 make filename identity, shape, status syntax, and the
//! whole-file word cap deterministic rather than review-only conventions.

use crate::markdown::{after, first_nonblank, headings, link_targets, names_a_record};
use crate::violation::{Violation, assert_clean};
use crate::{read, tree};

const ZONE: &str = "docs/decisions";
const TEMPLATE: &str = "docs/decisions/template.md";
const CAP: usize = 350;

/// A floor, so a walk that finds nothing fails instead of passing. The corpus
/// is 79 records; the floor is low enough to survive deliberate pruning and
/// high enough to catch a broken filter.
const FLOOR: usize = 70;
const SENTINELS: &[&str] = &["ADR-0001", "ADR-0074", "ADR-0079"];

const HEADINGS: &[&str] = &[
    "## Context and Problem Statement",
    "## Considered Options",
    "## Decision Outcome",
    "## Consequences",
    "## Status",
];

const STATUSES: &[&str] = &[
    "Ideation",
    "Proposed",
    "Accepted",
    "Implemented",
    "Deprecated",
    "Superseded",
    "Rejected",
];

fn records() -> Vec<String> {
    tree::names(ZONE)
        .into_iter()
        .filter(|name| name.starts_with("ADR-") && name.ends_with(".md"))
        .collect()
}

/// `ADR-0074-enforce-boundary-rules.md` names `0074`.
fn identifier(name: &str) -> &str {
    let rest = name.strip_prefix("ADR-").unwrap_or(name);
    rest.split('-').next().unwrap_or(rest)
}

/// Ids are unique and run without a gap from one. The sequence is a property of
/// the filenames alone, so it is checked once over the whole zone rather than
/// per record.
fn sequence(names: &[String]) -> Vec<Violation> {
    let mut found = Vec::new();
    let mut seen: Vec<&str> = Vec::new();
    let mut previous = 0_usize;
    for name in names {
        let path = format!("{ZONE}/{name}");
        let id = identifier(name);
        if seen.contains(&id) {
            found.push(Violation::whole(
                path.as_str(),
                format!("duplicate ADR id {id}"),
            ));
        }
        seen.push(id);
        let numeric = id.parse::<usize>().unwrap_or(0);
        if numeric != previous + 1 {
            found.push(Violation::whole(
                path.as_str(),
                format!("ADR ids are not contiguous after {previous:04}"),
            ));
        }
        previous = numeric;
    }
    found
}

/// The relative link destinations in the status evidence paragraph. An
/// absolute or external target is not a repository artifact and does not count.
fn relative_targets<'a>(lines: &[(usize, &'a str)]) -> Vec<&'a str> {
    lines
        .iter()
        .flat_map(|(_, line)| link_targets(line))
        .filter(|target| target.starts_with("./") || target.starts_with("../"))
        .collect()
}

fn record(path: &str, id: &str, text: &str) -> Vec<Violation> {
    let mut found = Vec::new();

    let title = text.lines().next().unwrap_or_default();
    if !title.starts_with(&format!("# ADR-{id}:")) {
        found.push(Violation::whole(path, "filename and title id disagree"));
    }

    if headings(text) != HEADINGS {
        found.push(Violation::whole(
            path,
            "headings differ from the five-section template",
        ));
    }

    let tail = after(text, "## Status");
    let status = first_nonblank(&tail).unwrap_or_default();
    if !STATUSES.contains(&status) {
        found.push(Violation::whole(
            path,
            format!("invalid first nonblank status value: {status}"),
        ));
    }

    let words = text.split_whitespace().count();
    if words > CAP {
        found.push(Violation::whole(
            path,
            format!("{words} words exceeds {CAP}"),
        ));
    }

    let targets = relative_targets(&tail);
    if status == "Superseded" && !targets.iter().any(|target| names_a_record(target)) {
        found.push(Violation::whole(
            path,
            "Superseded status lacks a relative successor link",
        ));
    }
    // Enactment evidence is the artifact that carries out the decision, so a
    // link to another record does not count: an "Amended by" or "Supersedes"
    // line alone would otherwise satisfy this gate and leave it inert.
    if status == "Implemented" && !targets.iter().any(|target| !names_a_record(target)) {
        found.push(Violation::whole(
            path,
            "Implemented status lacks a link to the artifact that enacts it",
        ));
    }

    found
}

fn template(text: &str) -> Vec<Violation> {
    let mut found = Vec::new();
    if headings(text) != HEADINGS {
        found.push(Violation::whole(
            TEMPLATE,
            "headings differ from the five-section template",
        ));
    }
    for status in STATUSES {
        if !text.contains(status) {
            found.push(Violation::whole(
                TEMPLATE,
                format!("advertised status set lacks {status}"),
            ));
        }
    }
    found
}

#[test]
fn every_adr_satisfies_the_contract() {
    let names = records();
    let mut found = sequence(&names);
    for name in &names {
        let path = format!("{ZONE}/{name}");
        found.extend(record(&path, identifier(name), &read(&path)));
    }
    assert_clean(&found);
}

#[test]
fn the_template_satisfies_the_contract() {
    assert_clean(&template(&read(TEMPLATE)));
}

/// The gate is worthless if the walk returns nothing, and a filter that
/// silently matches no file is indistinguishable from a clean corpus.
#[test]
fn the_corpus_is_large_enough_to_mean_something() {
    let names = records();
    assert!(
        names.len() >= FLOOR,
        "found {} decision records, expected at least {FLOOR}",
        names.len()
    );
    for sentinel in SENTINELS {
        assert!(
            names.iter().any(|name| name.starts_with(sentinel)),
            "{sentinel} was not recovered by the walk"
        );
    }
}

/// The cap is defined by `wc -w`, which splits on the C library's notion of
/// space; `split_whitespace` splits on the Unicode one. They agree on every
/// character in this corpus and diverge on the non-breaking spaces, so the
/// equivalence is asserted rather than assumed.
#[test]
fn word_count_measures_the_same_thing_as_wc() {
    let mut found = Vec::new();
    for name in records() {
        let path = format!("{ZONE}/{name}");
        for (number, line) in read(&path).lines().enumerate() {
            if let Some(odd) = line
                .chars()
                .find(|c| c.is_whitespace() && !c.is_ascii_whitespace())
            {
                found.push(Violation::at(
                    path.as_str(),
                    number + 1,
                    format!("{odd:?} counts as a word separator here but not to wc"),
                ));
            }
        }
    }
    assert_clean(&found);
}

// The tests below prove the gate can go red. They read no files, driving every
// rule from a doctored literal, which is what makes a green run above mean
// something.

const RECORD: &str = "# ADR-0001: A record

## Context and Problem Statement

Why this matters.

## Considered Options

- One
- Two

## Decision Outcome

Chosen option: one.

## Consequences

- Good: it works
- Bad: it costs

## Status

{status}
{tail}";

fn doctored(status: &str, tail: &str) -> String {
    RECORD.replace("{status}", status).replace("{tail}", tail)
}

fn judged(text: &str) -> Vec<Violation> {
    record("docs/decisions/ADR-0001-a-record.md", "0001", text)
}

#[test]
fn a_healthy_record_is_accepted() {
    assert_clean(&judged(&doctored("Accepted", "")));
}

#[test]
fn a_gap_in_the_id_sequence_is_a_violation() {
    let names = vec!["ADR-0001-a.md".to_string(), "ADR-0003-c.md".to_string()];
    let found = sequence(&names);
    assert_eq!(found.len(), 1, "{}", crate::violation::render(&found));
    assert!(found[0].to_string().contains("not contiguous after 0001"));
}

#[test]
fn a_duplicate_id_is_a_violation() {
    let names = vec!["ADR-0001-a.md".to_string(), "ADR-0001-b.md".to_string()];
    let found = sequence(&names);
    assert!(
        found.iter().any(|v| v.to_string().contains("duplicate")),
        "{}",
        crate::violation::render(&found)
    );
}

#[test]
fn a_title_naming_another_id_is_a_violation() {
    let text = doctored("Accepted", "").replace("# ADR-0001:", "# ADR-0002:");
    let found = judged(&text);
    assert_eq!(found.len(), 1, "{}", crate::violation::render(&found));
    assert!(found[0].to_string().contains("title id disagree"));
}

#[test]
fn a_missing_heading_is_a_violation() {
    let text = doctored("Accepted", "").replace("## Consequences\n", "");
    let found = judged(&text);
    assert!(
        found.iter().any(|v| v.to_string().contains("five-section")),
        "{}",
        crate::violation::render(&found)
    );
}

#[test]
fn a_reordered_heading_is_a_violation() {
    let text = doctored("Accepted", "")
        .replace("## Considered Options", "@@")
        .replace("## Decision Outcome", "## Considered Options")
        .replace("@@", "## Decision Outcome");
    let found = judged(&text);
    assert!(
        found.iter().any(|v| v.to_string().contains("five-section")),
        "{}",
        crate::violation::render(&found)
    );
}

#[test]
fn an_unknown_status_word_is_a_violation() {
    let found = judged(&doctored("Cooked", ""));
    assert_eq!(found.len(), 1, "{}", crate::violation::render(&found));
    assert!(found[0].to_string().contains("invalid first nonblank"));
}

#[test]
fn a_status_value_sharing_its_line_is_a_violation() {
    let found = judged(&doctored("Accepted today", ""));
    assert!(
        found
            .iter()
            .any(|v| v.to_string().contains("invalid first nonblank")),
        "{}",
        crate::violation::render(&found)
    );
}

fn padded(words: usize) -> String {
    let base = doctored("Accepted", "");
    let have = base.split_whitespace().count();
    format!("{base}\n{}", vec!["word"; words - have].join(" "))
}

#[test]
fn a_record_exactly_at_the_cap_is_accepted() {
    let text = padded(CAP);
    assert_eq!(text.split_whitespace().count(), CAP);
    assert_clean(&judged(&text));
}

#[test]
fn a_record_one_word_over_the_cap_is_a_violation() {
    let text = padded(CAP + 1);
    let found = judged(&text);
    assert_eq!(found.len(), 1, "{}", crate::violation::render(&found));
    assert!(found[0].to_string().contains("351 words exceeds 350"));
}

#[test]
fn a_superseded_record_without_a_successor_link_is_a_violation() {
    let found = judged(&doctored("Superseded", ""));
    assert_eq!(found.len(), 1, "{}", crate::violation::render(&found));
    assert!(found[0].to_string().contains("successor link"));
}

#[test]
fn a_superseded_record_naming_its_successor_is_accepted() {
    let tail = "\nSuperseded by [ADR-0002](./ADR-0002-next.md).\n";
    assert_clean(&judged(&doctored("Superseded", tail)));
}

#[test]
fn an_implemented_record_whose_only_link_is_an_adr_is_a_violation() {
    let tail = "\nSupersedes [ADR-0002](./ADR-0002-next.md).\n";
    let found = judged(&doctored("Implemented", tail));
    assert_eq!(found.len(), 1, "{}", crate::violation::render(&found));
    assert!(found[0].to_string().contains("artifact that enacts it"));
}

#[test]
fn an_implemented_record_linking_its_artifact_is_accepted() {
    let tail = "\nEnacted by [the gate](../../tests/repo_contracts/adrs.rs).\n";
    assert_clean(&judged(&doctored("Implemented", tail)));
}

#[test]
fn an_absolute_enactment_link_does_not_count() {
    let tail = "\nEnacted by [the gate](/tests/repo_contracts/adrs.rs).\n";
    let found = judged(&doctored("Implemented", tail));
    assert_eq!(found.len(), 1, "{}", crate::violation::render(&found));
}

#[test]
fn a_template_missing_a_status_word_is_a_violation() {
    let text = doctored("Accepted", "").replace("Accepted", "Agreed");
    let found = template(&text);
    assert!(
        found
            .iter()
            .any(|v| v.to_string().contains("lacks Accepted")),
        "{}",
        crate::violation::render(&found)
    );
}
