//! The collision audit ADR-0044 requires: the wrapper's claimed spellings meet
//! the child's inventory only where the CLI surface names the overlap.
//!
//! This reads two checked-in repository files and nothing else. It never spawns
//! the child and never uses the network, so it is a fact about this repository
//! rather than about the host; refreshing the fixture is a measurement, not a
//! test run. Reading a file is still the filesystem access the unit lane
//! forbids, which is what puts this in the integration lane.
//!
//! The crate root is `main.rs` inside the target directory rather than a
//! sibling `collision_audit.rs`, because a test target's root file is its own
//! crate root: `mod surface;` beside such a file would resolve to `tests/`, and
//! every helper would become a stray test binary.

mod audit;
mod inventory;
mod surface;

use std::fs;
use std::path::Path;

use audit::{FIXTURE, SURFACE, render};
use inventory::Inventory;
use surface::Surface;

/// A missing or unreadable input is a failure naming the path, never a skip. A
/// gate that can quietly decline to run is not a gate.
fn read(relative: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
    fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("{relative} is required by the collision audit: {err}"))
}

fn load_inventory() -> Inventory {
    inventory::parse(&read(FIXTURE)).unwrap_or_else(|found| panic!("{}", render(&found)))
}

fn load_surface() -> Surface {
    surface::parse(&read(SURFACE)).unwrap_or_else(|found| panic!("{}", render(&found)))
}

#[test]
fn fixture_is_well_formed() {
    let _ = load_inventory();
}

#[test]
fn surface_tables_are_well_formed() {
    let found = surface::contracts(&load_surface());
    assert!(found.is_empty(), "{}", render(&found));
}

#[test]
fn claimed_flags_match_the_child_inventory() {
    let found = audit::flags(&load_surface(), &load_inventory());
    assert!(found.is_empty(), "{}", render(&found));
}

#[test]
fn claimed_verbs_match_the_child_inventory() {
    let found = audit::verbs(&load_surface(), &load_inventory());
    assert!(found.is_empty(), "{}", render(&found));
}

#[test]
fn prose_datelines_match_the_fixture() {
    let found = audit::datelines(&read(SURFACE), &load_inventory());
    assert!(found.is_empty(), "{}", render(&found));
}

// The tests below prove the gate can go red. Until one of them exists, nothing
// has demonstrated that a clean run means anything: a parser that silently
// matched zero rows would pass every test above. They read no files, so they are
// unit tests by the evidence rule; splitting one module across two lanes to
// satisfy the taxonomy would cost more than the inaccuracy does.

const INVENTORY: &str = "\
child_version: \"2.1.220\"
measured_on: \"2026-08-06\"
source_command: \"claude --help\"
top_level_flags:
    - \"--help\"
    - \"--settings\"
    - \"--verbose\"
    - \"--version\"
    - \"-h\"
    - \"-v\"
top_level_verbs:
    - \"auth\"
    - \"doctor\"
aliases:
    \"-v\": \"--version\"
subcommand_flags:
    doctor:
        - \"--help\"
";

/// Assembles a surface document from the three table bodies, so each negative
/// test doctors exactly one of them.
fn document(flags: &str, verbs: &str, overlaps: &str) -> String {
    format!(
        "## Wrapper-owned flags\n\n\
        | Flag | Meaning | Why the wrapper claims it | Child status |\n\
        | ---- | ------- | ------------------------- | ------------ |\n\
        {flags}\n\
        ## Wrapper verbs\n\n\
        | Verb | Purpose | Grammar specified in |\n\
        | ---- | ------- | -------------------- |\n\
        {verbs}\n\
        | Child verb | Shared surface | Resolution | Reason |\n\
        | ---------- | -------------- | ---------- | ------ |\n\
        {overlaps}"
    )
}

const DOCTOR_VERB: &str = "| `doctor` | p | g |\n\n";
const DOCTOR_COMPOSED: &str = "| `doctor` | s | Composed | r |\n";

fn parsed(flags: &str, verbs: &str, overlaps: &str) -> Surface {
    surface::parse(&document(flags, verbs, overlaps))
        .unwrap_or_else(|found| panic!("the doctored document should parse: {}", render(&found)))
}

#[test]
fn a_free_claim_over_an_inventoried_flag_is_a_violation() {
    let surface = parsed(
        "| `--verbose` | m | w | Free |\n\n",
        DOCTOR_VERB,
        DOCTOR_COMPOSED,
    );
    let found = audit::flags(
        &surface,
        &inventory::parse(INVENTORY).expect("fixture parses"),
    );
    assert_eq!(found.len(), 1, "{}", render(&found));
    assert!(
        found[0].to_string().contains("`--verbose`"),
        "{}",
        render(&found)
    );
}

#[test]
fn a_collision_claim_over_an_absent_flag_is_a_violation() {
    let surface = parsed(
        "| `--quiet` | m | w | Collides by design |\n\n",
        DOCTOR_VERB,
        DOCTOR_COMPOSED,
    );
    let found = audit::flags(
        &surface,
        &inventory::parse(INVENTORY).expect("fixture parses"),
    );
    assert_eq!(found.len(), 1, "{}", render(&found));
    assert!(
        found[0].to_string().contains("`--quiet`"),
        "{}",
        render(&found)
    );
}

#[test]
fn claiming_a_child_alias_is_a_violation() {
    let surface = parsed(
        "| `-v` | m | w | Collides by design |\n\n",
        DOCTOR_VERB,
        DOCTOR_COMPOSED,
    );
    let found = audit::flags(
        &surface,
        &inventory::parse(INVENTORY).expect("fixture parses"),
    );
    assert_eq!(found.len(), 1, "{}", render(&found));
    assert!(
        found[0].to_string().contains("change a token's meaning"),
        "{}",
        render(&found)
    );
}

#[test]
fn a_renamed_verb_that_is_still_claimed_is_a_violation() {
    let surface = parsed(
        "| `--quiet` | m | w | Free |\n\n",
        "| `auth` | p | g |\n\n",
        "| `auth` | s | Renamed | r |\n",
    );
    let found = audit::verbs(
        &surface,
        &inventory::parse(INVENTORY).expect("fixture parses"),
    );
    assert_eq!(found.len(), 1, "{}", render(&found));
    assert!(
        found[0].to_string().contains("still claims it"),
        "{}",
        render(&found)
    );
}

#[test]
fn an_unrecognized_child_status_clause_is_rejected() {
    // The status a reviewer is most likely to write by accident is one that is
    // neither of the two forms. It must fail, not fall back to free.
    let found = surface::parse(&document(
        "| `--quiet` | m | w | Unclear |\n\n",
        DOCTOR_VERB,
        DOCTOR_COMPOSED,
    ))
    .err()
    .unwrap_or_else(|| panic!("an unrecognized status clause must not parse"));
    assert!(
        found
            .iter()
            .any(|v| v.to_string().contains("unrecognized child-status clause")),
        "{}",
        render(&found)
    );
}

#[test]
fn a_negated_collision_clause_is_rejected() {
    // The one wrong reading a substring test produces: `Does not collide` is a
    // free claim, and reading it as a collision would keep an inventoried row
    // green on a statement of the opposite. It is not in the grammar, so it
    // fails rather than resolving either way.
    let found = surface::parse(&document(
        "| `--verbose` | m | w | Does not collide |\n\n",
        DOCTOR_VERB,
        DOCTOR_COMPOSED,
    ))
    .err()
    .unwrap_or_else(|| panic!("a negated collision clause must not parse"));
    assert!(
        found
            .iter()
            .any(|v| v.to_string().contains("unrecognized child-status clause")),
        "{}",
        render(&found)
    );
}

#[test]
fn a_flags_table_missing_a_column_is_rejected() {
    let document = document(
        "| `--quiet` | m | w | Free |\n\n",
        DOCTOR_VERB,
        DOCTOR_COMPOSED,
    )
    .replace("| Why the wrapper claims it ", "");
    let found = surface::parse(&document)
        .err()
        .unwrap_or_else(|| panic!("a flags table missing a column must not parse"));
    assert!(
        found
            .iter()
            .any(|v| v.to_string().contains("Wrapper-owned flags")),
        "{}",
        render(&found)
    );
}

#[test]
fn an_empty_flag_list_is_rejected() {
    // Anchored on the key so the identically spelled entry under
    // `subcommand_flags` keeps its indentation and the fixture stays parseable.
    let fixture = INVENTORY.replace("top_level_flags:\n    - \"--help\"\n", "top_level_flags:\n");
    let found = inventory::parse(&fixture)
        .err()
        .unwrap_or_else(|| panic!("a fixture missing a sentinel must not parse"));
    assert!(
        found
            .iter()
            .any(|v| v.to_string().contains("pass vacuously")),
        "{}",
        render(&found)
    );
}
