//! ADR-0077 makes `milestones.md` the only status surface and fixes the
//! slice entry shape. This gate reads the whole zone, so drift that spans two
//! files cannot hide between them.

use crate::markdown::{first_nonblank, headings, link_targets, section};
use crate::violation::{Violation, assert_clean};
use crate::{read, tree};

const MILESTONES: &str = "docs/plan/milestones.md";
const SLICES: &str = "docs/plan/slices";
const QUESTIONS: &str = "docs/plan/open-questions.md";

/// A floor and sentinels, so a list that parses to nothing fails rather than
/// passing. There are twelve lines today.
const FLOOR: usize = 11;
const SENTINELS: &[&str] = &["001", "009"];

/// The two sections, live work first. A terminal status belongs only under the
/// second, and the move is one-way.
const IN_FLIGHT: &str = "## in flight";
const CLOSED: &str = "## closed";

/// The field separator in the milestone grammar. A note may not contain it, so
/// the line parses to a fixed field count.
const FIELD: &str = " — ";

/// The shape configs markdownlint applies to these documents. Each states a
/// heading array this file also fixes in a constant, so the gate asserts the two
/// agree rather than letting a second owner drift.
const MILESTONES_SHAPE: &str = ".markdownlint/milestones.markdownlint-cli2.jsonc";
const SLICE_SHAPE: &str = ".markdownlint/slice-readme.markdownlint-cli2.jsonc";

/// Merged over every shape, so it may not name `MD043` at any value.
const PROJECT_CONFIG: &str = ".markdownlint-cli2.jsonc";

/// Every shape config lives here, and the hook file is what points at one. A
/// config neither side names is an array nothing reads.
const SHAPE_DIR: &str = ".markdownlint";
const HOOKS: &str = ".pre-commit-config.yaml";

/// The status surface's own headings, which the slice shape does not cover.
const SECTIONS: &[&str] = &["# Milestones", IN_FLIGHT, CLOSED];

const HEADINGS: &[&str] = &[
    "## Goal",
    "## Appetite",
    "## Core",
    "## In scope",
    "## Out of scope",
    "## Governed by",
    "## Acceptance",
    "## Rabbit holes",
    "## Done when",
    "## Revisions",
];

const STATUSES: &[&str] = &["shaped", "active", "done", "cut", "reshaped"];
const TERMINAL: &[&str] = &["done", "cut", "reshaped"];

/// The five adopted EARS shapes, complex being the state-driven form with a
/// trigger clause inside it. Each needs `shall` and a response, so a bare
/// trigger such as "When the cache misses." is not an assertion. The system
/// noun is never fixed: the child, the report, and the wrapper are all
/// legitimate subjects.
const EARS: &[&str] = &[
    "- The * shall *",
    "- While *, * shall *",
    "- When *, * shall *",
    "- Where *, * shall *",
    "- If *, then * shall *",
];

#[derive(Debug, Clone)]
struct Row {
    id: String,
    status: String,
    appetite: String,
    dir: String,
    closed: bool,
    /// The ids the note names as predecessors, which order `## in flight`.
    after: Vec<String>,
}

/// The note's optional leading clause, which makes execution order checkable
/// rather than asserted. Only the first clause is read, so prose after the
/// semicolon stays free text.
const AFTER: &str = "after ";

/// Shell-glob matching with `*` only, because the shapes above were shell
/// `case` patterns and reproducing them exactly is the point.
fn glob(pattern: &str, text: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();
    let Some(mut rest) = text.strip_prefix(parts[0]) else {
        return false;
    };
    let last = parts.len() - 1;
    for (index, part) in parts.iter().enumerate().skip(1) {
        if index == last {
            return part.is_empty() || rest.ends_with(part);
        }
        let Some(at) = rest.find(part) else {
            return false;
        };
        rest = &rest[at + part.len()..];
    }
    true
}

fn is_identifier(text: &str) -> bool {
    text.len() == 3 && text.chars().all(|c| c.is_ascii_digit())
}

/// `<id> <slug>` splits on one space, so a slug that swallowed the separator is
/// caught here rather than becoming an unmatched directory later.
fn identity(head: &str) -> Option<(String, String)> {
    let (id, slug) = head.split_once(' ')?;
    let shaped = is_identifier(id)
        && !slug.is_empty()
        && slug
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    shaped.then(|| (id.to_string(), slug.to_string()))
}

/// The predecessor ids in a note, read from its first clause only. A note that
/// does not open with the clause names none, which is how a slice with no
/// predecessor spells the absence.
fn predecessors(note: Option<&&str>) -> Vec<String> {
    let Some(clause) = note.and_then(|text| text.split(';').next()) else {
        return Vec::new();
    };
    let Some(ids) = clause.strip_prefix(AFTER) else {
        return Vec::new();
    };
    ids.split(|c: char| !c.is_ascii_digit())
        .filter(|token| is_identifier(token))
        .map(str::to_string)
        .collect()
}

/// Parses the two-section list. The grammar is fixed so the status surface stays
/// machine-readable: `- <id> <slug> — <status> — <appetite>[ — <note>]`, and
/// nothing else on the line.
fn milestones(text: &str) -> (Vec<Row>, Vec<Violation>) {
    let mut rows: Vec<Row> = Vec::new();
    let mut found = Vec::new();
    let mut counts: Vec<(String, usize)> = Vec::new();
    let mut closed: Option<bool> = None;
    let mut previous = 0_usize;

    for line in text.lines() {
        // Any other heading ends the list rather than being read as part of it,
        // so a third section cannot smuggle entries past the ordering checks.
        if line.starts_with("## ") {
            closed = match line {
                IN_FLIGHT => Some(false),
                CLOSED => Some(true),
                _ => None,
            };
            previous = 0;
            continue;
        }
        let Some(closed) = closed else { continue };
        if line.trim().is_empty() {
            continue;
        }

        let Some(entry) = line.strip_prefix("- ") else {
            found.push(Violation::whole(
                MILESTONES,
                format!("line in a milestone section is not an entry: {line}"),
            ));
            continue;
        };
        let fields: Vec<&str> = entry.split(FIELD).collect();
        if !(3..=4).contains(&fields.len()) {
            found.push(Violation::whole(
                MILESTONES,
                format!("milestone entry is not the fixed grammar: {line}"),
            ));
            continue;
        }
        let Some((id, slug)) = identity(fields[0]) else {
            found.push(Violation::whole(
                MILESTONES,
                format!("milestone entry does not open with an id and a slug: {line}"),
            ));
            continue;
        };

        // Counted before any other check so a duplicate is caught even when its
        // entry is otherwise malformed. Later entries replace earlier ones
        // below, so without this a second contradictory line would be invisible.
        match counts.iter_mut().find(|(seen, _)| *seen == id) {
            Some((_, count)) => *count += 1,
            None => counts.push((id.clone(), 1)),
        }

        // Ascending inside `## closed`, which is a ledger: the run stays
        // scannable and a misfiled entry is visible without reading every line.
        // `## in flight` is ordered by execution instead, and the predecessor
        // pass below is what proves it.
        let numeric = id.parse::<usize>().unwrap_or(0);
        if closed && numeric <= previous {
            found.push(Violation::whole(
                MILESTONES,
                format!("milestone {id} is out of ascending id order in its section"),
            ));
        }
        previous = numeric;

        let status = fields[1];
        if !STATUSES.contains(&status) {
            found.push(Violation::whole(
                MILESTONES,
                format!("milestone {id} has invalid status {status}"),
            ));
        } else if TERMINAL.contains(&status) != closed {
            found.push(Violation::whole(
                MILESTONES,
                format!("milestone {id} carries {status} under the wrong section"),
            ));
        }

        let dir = format!("{id}-{slug}");
        if !tree::is_file(&format!("{SLICES}/{dir}/README.md")) {
            found.push(Violation::whole(
                MILESTONES,
                format!("milestone {id} names no slice directory {dir}"),
            ));
        }

        rows.retain(|row| row.id != id);
        rows.push(Row {
            id,
            status: status.to_string(),
            appetite: fields[2].to_string(),
            dir,
            closed,
            after: predecessors(fields.get(3)),
        });
    }

    for (id, count) in counts {
        if count > 1 {
            found.push(Violation::whole(
                MILESTONES,
                format!("milestone {id} has {count} lines; one status surface means one line"),
            ));
        }
    }

    // `## in flight` reads in execution order, so the top line is the next slice
    // to pick up. That is only true while every predecessor a note names is
    // already closed or already listed above, which is what this proves. Closed
    // work is finished, so it satisfies any predecessor from either section.
    let live: Vec<&Row> = rows.iter().filter(|row| !row.closed).collect();
    for (index, row) in live.iter().enumerate() {
        for predecessor in &row.after {
            let known = rows.iter().find(|other| other.id == *predecessor);
            let Some(known) = known else {
                found.push(Violation::whole(
                    MILESTONES,
                    format!(
                        "milestone {} names predecessor {predecessor}, which has no line",
                        row.id
                    ),
                ));
                continue;
            };
            if known.closed {
                continue;
            }
            let above = live
                .iter()
                .take(index)
                .any(|earlier| earlier.id == *predecessor);
            if !above {
                found.push(Violation::whole(
                    MILESTONES,
                    format!(
                        "milestone {} is above its predecessor {predecessor}, \
                        so in flight is not in execution order",
                        row.id
                    ),
                ));
            }
        }
    }

    (rows, found)
}

/// A document carries no lint configuration of its own: the shape lives in
/// `.markdownlint/` and a hook entry applies it. A leftover configure comment
/// would override that shape from inside the file, silently and per document.
fn no_inline_configuration(path: &str, text: &str) -> Vec<Violation> {
    text.lines()
        .enumerate()
        .filter(|(_, line)| line.contains("markdownlint-configure-file"))
        .map(|(index, _)| {
            Violation::at(
                path,
                index + 1,
                "heading shapes live in .markdownlint/, not in the document",
            )
        })
        .collect()
}

fn slice_directories(rows: &[Row]) -> (Vec<String>, Vec<Violation>) {
    let mut found = Vec::new();
    let mut matched = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    let mut previous = 0_usize;

    for dir in tree::directories(SLICES) {
        let path = format!("{SLICES}/{dir}");
        let shaped = dir.len() >= 5
            && is_identifier(&dir[..3])
            && dir.as_bytes()[3] == b'-'
            && dir[4..]
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
        if !shaped {
            found.push(Violation::whole(path, "invalid slice directory name"));
            continue;
        }

        let id = dir[..3].to_string();
        let numeric = id.parse::<usize>().unwrap_or(0);
        if seen.contains(&id) || numeric != previous + 1 {
            found.push(Violation::whole(
                path.as_str(),
                "reused or non-contiguous slice id",
            ));
        }
        seen.push(id.clone());
        previous = numeric;

        // Without the row there is no appetite or status to check this slice
        // against, and continuing would mask every later slice's defects behind
        // this one.
        if rows.iter().any(|row| row.id == id && row.dir == dir) {
            matched.push(dir);
        } else {
            found.push(Violation::whole(path, "no unique matching milestone line"));
        }
    }

    (matched, found)
}

fn shape(path: &str, readme: &str) -> Vec<Violation> {
    let mut found = Vec::new();
    if headings(readme) != HEADINGS {
        found.push(Violation::whole(
            path,
            "headings differ from the fixed slice shape",
        ));
    }
    if readme.lines().any(|line| line == "## Status") {
        found.push(Violation::whole(path, "slices may not carry local status"));
    }
    found
}

/// The whole appetite is compared, not just its number: a slice reading
/// "4 weeks" against a milestone reading "4 sessions" is the two-surface drift
/// the plan zone exists to prevent.
fn appetite(path: &str, readme: &str, row: &Row) -> Vec<Violation> {
    let stated = first_nonblank(&section(readme, "## Appetite", "## Core"))
        .map(|line| line.strip_suffix('.').unwrap_or(line))
        .unwrap_or_default();
    if stated == row.appetite {
        return Vec::new();
    }
    vec![Violation::whole(
        path,
        format!(
            "appetite {stated} does not match milestone {} appetite {}",
            row.id, row.appetite
        ),
    )]
}

fn governed_by(path: &str, dir: &str, readme: &str) -> Vec<Violation> {
    let base = format!("{SLICES}/{dir}");
    let mut found = Vec::new();
    for (_, line) in section(readme, "## Governed by", "## Acceptance") {
        for target in link_targets(line) {
            let target = target.split('#').next().unwrap_or_default();
            if target.is_empty() || target.starts_with("http://") || target.starts_with("https://")
            {
                continue;
            }
            if !tree::is_file(&tree::normalize(&base, target)) {
                found.push(Violation::whole(
                    path,
                    format!("Governed by target is missing or not a file: {target}"),
                ));
            }
        }
    }
    found
}

fn acceptance(path: &str, readme: &str) -> Vec<Violation> {
    section(readme, "## Acceptance", "## Rabbit holes")
        .into_iter()
        .filter(|(_, line)| !line.trim().is_empty())
        .filter(|(_, line)| !EARS.iter().any(|shape| glob(shape, line)))
        .map(|(number, line)| {
            Violation::at(
                path,
                number,
                format!("acceptance line is not an assertion-only EARS form: {line}"),
            )
        })
        .collect()
}

fn rabbit_holes(path: &str, readme: &str) -> Vec<Violation> {
    section(readme, "## Rabbit holes", "## Done when")
        .into_iter()
        .filter(|(_, line)| line.starts_with("- ") && !line.contains("escape:"))
        .map(|(number, line)| {
            Violation::at(path, number, format!("rabbit hole lacks escape: {line}"))
        })
        .collect()
}

/// Cross-session progress is allowed only while work can advance. Its file
/// stays a title plus flat checklist, never a second requirements surface.
fn tasks(path: &str, text: &str, status: &str) -> Vec<Violation> {
    let mut found = Vec::new();
    if TERMINAL.contains(&status) {
        found.push(Violation::whole(
            path,
            "terminal milestone status forbids tasks.md",
        ));
    }
    let mut items = 0;
    let mut malformed = false;
    for (index, line) in text.lines().enumerate() {
        match index {
            0 => malformed |= line != "# Tasks",
            1 => malformed |= !line.is_empty(),
            _ if line.is_empty() => {}
            _ if is_checkbox(line) => items += 1,
            _ => malformed = true,
        }
    }
    if malformed || items == 0 {
        found.push(Violation::whole(
            path,
            "expected only # Tasks and flat checkbox items",
        ));
    }
    found
}

fn is_checkbox(line: &str) -> bool {
    let Some(rest) = line.strip_prefix("- [") else {
        return false;
    };
    let mark = rest.as_bytes().first().copied();
    matches!(mark, Some(b' ' | b'x')) && rest.len() > 3 && &rest[1..3] == "] "
}

/// Requirements need an explicit reasoned gate and replace slice acceptance;
/// they never duplicate assertions in two files.
fn requirements(readme_path: &str, readme: &str, path: &str) -> Vec<Violation> {
    let mut found = Vec::new();
    let lines: Vec<&str> = readme.lines().collect();
    let gated = lines
        .windows(2)
        .any(|pair| is_gate(pair[0]) && pair[1] == "## Acceptance");
    if !gated {
        found.push(Violation::whole(
            path,
            "requirements.md lacks a reasoned gate immediately above Acceptance",
        ));
    }
    if section(readme, "## Acceptance", "## Rabbit holes")
        .iter()
        .any(|(_, line)| line.starts_with("- "))
    {
        found.push(Violation::whole(
            readme_path,
            "README Acceptance must be empty when requirements.md exists",
        ));
    }
    found
}

fn is_gate(line: &str) -> bool {
    let Some(inner) = line
        .strip_prefix("<!--")
        .and_then(|s| s.strip_suffix("-->"))
    else {
        return false;
    };
    let Some(reason) = inner.trim_start().strip_prefix("requirements-gate:") else {
        return false;
    };
    !reason.trim_start().is_empty()
}

/// Every candidate heading is judged, not only the well-formed ones: a mistyped
/// or reused id silently removes a blocker from machine-visible plan state, and
/// matching the strict form alone would skip exactly the malformed heading the
/// gate exists to catch.
fn questions(text: &str) -> Vec<Violation> {
    let mut found = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    let mut active = false;
    let mut id = String::new();
    let mut blocks = 0;
    let mut raised = 0;
    let mut exits = 0;

    let finish = |found: &mut Vec<Violation>, active: bool, id: &str, b, r, e| {
        if active && (b != 1 || r != 1 || e != 1) {
            found.push(Violation::whole(
                QUESTIONS,
                format!("Q-{id} needs exactly one Blocks, one Raised, and one permitted Exit"),
            ));
        }
    };

    for line in text.lines() {
        if line.starts_with("## ") {
            finish(&mut found, active, &id, blocks, raised, exits);
            active = false;
            if !line.starts_with("## Q") {
                continue;
            }
            if !is_question_heading(line) {
                found.push(Violation::whole(
                    QUESTIONS,
                    format!("question heading is not the stable Q-NNN form: {line}"),
                ));
                continue;
            }
            id = line.split_whitespace().nth(1).unwrap_or_default()[2..].to_string();
            if seen.contains(&id) {
                found.push(Violation::whole(
                    QUESTIONS,
                    format!("question id Q-{id} is reused; ids are stable and unique"),
                ));
            }
            seen.push(id.clone());
            active = true;
            blocks = 0;
            raised = 0;
            exits = 0;
            continue;
        }

        let field = ["Blocks:", "Raised:", "Exit:"]
            .iter()
            .any(|name| line.starts_with(name));
        if field && !active {
            found.push(Violation::whole(
                QUESTIONS,
                format!("field {line} belongs to no question block"),
            ));
            continue;
        }
        if !active {
            continue;
        }
        if line.len() > "Blocks: ".len() && line.starts_with("Blocks: ") {
            blocks += 1;
        } else if line.len() > "Raised: ".len() && line.starts_with("Raised: ") {
            raised += 1;
        } else if ["Exit: ADR,", "Exit: slice revision,", "Exit: measurement,"]
            .iter()
            .any(|prefix| line.starts_with(prefix))
        {
            exits += 1;
        }
    }
    finish(&mut found, active, &id, blocks, raised, exits);
    found
}

fn is_question_heading(line: &str) -> bool {
    let Some(rest) = line.strip_prefix("## Q-") else {
        return false;
    };
    rest.len() >= 3 && is_identifier(&rest[..3]) && (rest.len() == 3 || rest.as_bytes()[3] == b' ')
}

#[test]
fn the_plan_zone_satisfies_the_contract() {
    let text = read(MILESTONES);
    let (rows, mut found) = milestones(&text);
    found.extend(no_inline_configuration(MILESTONES, &text));
    found.extend(crate::shape::matches(
        MILESTONES_SHAPE,
        &read(MILESTONES_SHAPE),
        SECTIONS,
    ));

    // The status surface's own headings, which no other check here fixes.
    if headings(&text) != [IN_FLIGHT, CLOSED] {
        found.push(Violation::whole(
            MILESTONES,
            "the two sections are the whole document, live work first",
        ));
    }

    found.extend(crate::shape::absent_from_project_config(
        PROJECT_CONFIG,
        &read(PROJECT_CONFIG),
    ));
    found.extend(crate::shape::applied_by_a_hook(SHAPE_DIR, &read(HOOKS)));

    let mut slice_shape: Vec<&str> = vec!["*"];
    slice_shape.extend(HEADINGS);
    found.extend(crate::shape::matches(
        SLICE_SHAPE,
        &read(SLICE_SHAPE),
        &slice_shape,
    ));

    let (dirs, directory_findings) = slice_directories(&rows);
    found.extend(directory_findings);

    for dir in &dirs {
        let row = rows
            .iter()
            .find(|row| &row.dir == dir)
            .expect("a matched directory has its row");
        let path = format!("{SLICES}/{dir}/README.md");
        let readme = read(&path);

        let shape_findings = shape(&path, &readme);
        let shaped = shape_findings.is_empty();
        found.extend(shape_findings);
        found.extend(no_inline_configuration(&path, &readme));
        found.extend(appetite(&path, &readme, row));

        // A wrong heading list makes every section range below run past its
        // end, so the section-scoped checks only run once the shape is known
        // good. The sibling-file gates do not depend on the shape and always
        // run.
        if shaped {
            found.extend(governed_by(&path, dir, &readme));
            found.extend(acceptance(&path, &readme));
            found.extend(rabbit_holes(&path, &readme));
        }

        let tasks_path = format!("{SLICES}/{dir}/tasks.md");
        if tree::is_file(&tasks_path) {
            found.extend(tasks(&tasks_path, &read(&tasks_path), &row.status));
        }
        let requirements_path = format!("{SLICES}/{dir}/requirements.md");
        if tree::is_file(&requirements_path) {
            found.extend(requirements(&path, &readme, &requirements_path));
        }
    }

    // design.md is never an allowed sibling: design belongs in the living owner.
    for path in tree::files_named(SLICES, "design.md") {
        found.push(Violation::whole(
            path,
            "design.md is forbidden in the plan zone",
        ));
    }

    if rows.len() != tree::directories(SLICES).len() {
        found.push(Violation::whole(
            MILESTONES,
            "milestone line count and slice directory count differ",
        ));
    }

    found.extend(questions(&read(QUESTIONS)));
    assert_clean(&found);
}

#[test]
fn the_milestone_list_parsed_to_every_line() {
    let (rows, _) = milestones(&read(MILESTONES));
    assert!(
        rows.len() >= FLOOR,
        "parsed {} milestone lines, expected at least {FLOOR}",
        rows.len()
    );
    for sentinel in SENTINELS {
        let row = rows
            .iter()
            .find(|row| row.id == *sentinel)
            .unwrap_or_else(|| panic!("milestone {sentinel} was not recovered"));
        assert!(
            row.dir.starts_with(sentinel),
            "milestone {sentinel} recovered no matching directory"
        );
    }
}

/// The plan-zone links resolve lexically rather than through `canonicalize`,
/// which is equivalent to `realpath -m` only while no component is a symlink.
#[test]
fn the_plan_zone_contains_no_symlink() {
    assert_eq!(tree::symlinks("docs/plan"), Vec::<String>::new());
}

// The tests below prove the gate can go red, driven from literals.

const SLICE: &str = "# A slice

## Goal

Do the thing.

## Appetite

1 implementation session.

## Core

The core of it.

## In scope

- One.

## Out of scope

- Two.

## Governed by

- [AGENTS.md](../../../../AGENTS.md)

## Acceptance

{acceptance}

## Rabbit holes

{rabbit}

## Done when

It is done.

## Revisions

None.
";

fn slice(acceptance: &str, rabbit: &str) -> String {
    SLICE
        .replace("{acceptance}", acceptance)
        .replace("{rabbit}", rabbit)
}

fn healthy() -> String {
    slice(
        "- The wrapper shall pass argv through.",
        "- Gold plating; escape: cut from the end of In scope.",
    )
}

fn row(status: &str, appetite: &str) -> Row {
    Row {
        id: "001".to_string(),
        status: status.to_string(),
        appetite: appetite.to_string(),
        dir: "001-a-slice".to_string(),
        closed: TERMINAL.contains(&status),
        after: Vec::new(),
    }
}

/// Real slice directories, so a rule under test fails on its own account rather
/// than on a name that resolves to nothing.
const LIST: &str = "# Milestones\n\
    \n\
    ## in flight\n\
    \n\
    - 009 release-readiness — shaped — 1 implementation session\n\
    \n\
    ## closed\n\
    \n\
    - 001 native-passthrough-foundation — done — 4 implementation sessions\n";

/// The live entry, so a duplicate can be built from the fixture rather than
/// hand-written twice.
const NINE: &str = "- 009 release-readiness — shaped — 1 implementation session\n";

/// A live predecessor pair, so execution order can be built from the fixture in
/// either direction. The descending ids are the point: they are legal above,
/// and only the note decides which line may come first.
const THIRTEEN: &str = "- 013 token-account-lifecycle — shaped — 2 implementation sessions\n";
const FOUR: &str =
    "- 004 profile-composition — shaped — 2 implementation sessions — after 013; the rung\n";

#[test]
fn a_healthy_slice_is_accepted() {
    let readme = healthy();
    let mut found = shape("r.md", &readme);
    found.extend(appetite(
        "r.md",
        &readme,
        &row("active", "1 implementation session"),
    ));
    found.extend(acceptance("r.md", &readme));
    found.extend(rabbit_holes("r.md", &readme));
    assert_clean(&found);
}

#[test]
fn a_healthy_milestone_list_is_accepted() {
    let (rows, found) = milestones(LIST);
    assert_clean(&found);
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().any(|row| row.id == "001" && row.closed));
}

#[test]
fn a_status_outside_the_vocabulary_is_a_violation() {
    let text = LIST.replace("— shaped —", "— finished —");
    let (_, found) = milestones(&text);
    assert!(
        found
            .iter()
            .any(|v| v.to_string().contains("invalid status")),
        "{}",
        crate::violation::render(&found)
    );
}

#[test]
fn a_terminal_status_left_in_flight_is_a_violation() {
    let text = LIST.replace(
        "009 release-readiness — shaped",
        "009 release-readiness — done",
    );
    let (_, found) = milestones(&text);
    assert_eq!(found.len(), 1, "{}", crate::violation::render(&found));
    assert!(found[0].to_string().contains("under the wrong section"));
}

#[test]
fn a_live_status_under_closed_is_a_violation() {
    let text = LIST.replace(
        "001 native-passthrough-foundation — done",
        "001 native-passthrough-foundation — active",
    );
    let (_, found) = milestones(&text);
    assert_eq!(found.len(), 1, "{}", crate::violation::render(&found));
    assert!(found[0].to_string().contains("under the wrong section"));
}

#[test]
fn an_entry_naming_no_slice_directory_is_a_violation() {
    let text = LIST.replace("release-readiness", "no-such-slice");
    let (_, found) = milestones(&text);
    assert_eq!(found.len(), 1, "{}", crate::violation::render(&found));
    assert!(found[0].to_string().contains("names no slice directory"));
}

#[test]
fn an_entry_whose_id_disagrees_with_its_slug_is_a_violation() {
    let text = LIST.replace("- 009 release-readiness", "- 008 release-readiness");
    let (_, found) = milestones(&text);
    assert!(
        found
            .iter()
            .any(|v| v.to_string().contains("names no slice directory")),
        "{}",
        crate::violation::render(&found)
    );
}

#[test]
fn an_entry_outside_the_fixed_grammar_is_a_violation() {
    let text = LIST.replace(
        "- 009 release-readiness — shaped — 1 implementation session",
        "- 009 release-readiness is shaped",
    );
    let (_, found) = milestones(&text);
    assert_eq!(found.len(), 1, "{}", crate::violation::render(&found));
    assert!(found[0].to_string().contains("not the fixed grammar"));
}

#[test]
fn a_line_in_a_section_that_is_not_an_entry_is_a_violation() {
    let text = LIST.replace("## closed\n", "## closed\n\nA stray paragraph.\n");
    let (_, found) = milestones(&text);
    assert_eq!(found.len(), 1, "{}", crate::violation::render(&found));
    assert!(found[0].to_string().contains("is not an entry"));
}

#[test]
fn descending_ids_within_closed_are_a_violation() {
    let text = LIST.replace(
        "- 001 native-passthrough-foundation — done — 4 implementation sessions\n",
        "- 003 native-child-exec — done — 2 implementation sessions\n\
        - 002 secure-session-storage — done — 3 implementation sessions\n",
    );
    let (_, found) = milestones(&text);
    assert_eq!(found.len(), 1, "{}", crate::violation::render(&found));
    assert!(found[0].to_string().contains("ascending id order"));
}

#[test]
fn descending_ids_within_in_flight_are_accepted() {
    let text = LIST.replace(NINE, &format!("{THIRTEEN}{FOUR}"));
    let (_, found) = milestones(&text);
    assert_clean(&found);
}

#[test]
fn an_entry_above_its_live_predecessor_is_a_violation() {
    let text = LIST.replace(NINE, &format!("{FOUR}{THIRTEEN}"));
    let (_, found) = milestones(&text);
    assert_eq!(found.len(), 1, "{}", crate::violation::render(&found));
    assert!(found[0].to_string().contains("execution order"));
}

#[test]
fn a_closed_predecessor_orders_nothing() {
    let text = LIST.replace(
        NINE,
        "- 009 release-readiness — shaped — 1 implementation session — after 001\n",
    );
    let (_, found) = milestones(&text);
    assert_clean(&found);
}

#[test]
fn a_predecessor_with_no_line_is_a_violation() {
    let text = LIST.replace(
        NINE,
        "- 009 release-readiness — shaped — 1 implementation session — after 002 and 003\n",
    );
    let (_, found) = milestones(&text);
    assert_eq!(found.len(), 2, "{}", crate::violation::render(&found));
    assert!(found.iter().all(|v| v.to_string().contains("has no line")));
}

#[test]
fn an_entry_under_a_third_section_is_not_read_as_status() {
    let text = format!("{LIST}\n## backlog\n\n- 002 secure-session-storage — shaped — 3\n");
    let (rows, found) = milestones(&text);
    assert_clean(&found);
    assert_eq!(rows.len(), 2, "a third section must carry no status");
}

#[test]
fn two_lines_for_one_id_is_a_violation() {
    let text = LIST.replace(NINE, &format!("{NINE}{NINE}"));
    let (_, found) = milestones(&text);
    assert!(
        found.iter().any(|v| v.to_string().contains("has 2 lines")),
        "{}",
        crate::violation::render(&found)
    );
}

#[test]
fn an_inline_configure_comment_is_a_violation() {
    let text = format!(
        "# Milestones\n\n<!-- markdownlint-configure-file {{ \"MD043\": {{}} }} -->\n{LIST}"
    );
    let found = no_inline_configuration("m.md", &text);
    assert_eq!(found.len(), 1, "{}", crate::violation::render(&found));
    assert!(found[0].to_string().contains("live in .markdownlint/"));
}

#[test]
fn a_document_without_inline_configuration_is_accepted() {
    assert_clean(&no_inline_configuration("m.md", LIST));
}

#[test]
fn the_slice_shape_is_the_star_plus_the_heading_constant() {
    let mut expected: Vec<&str> = vec!["*"];
    expected.extend(HEADINGS);
    assert_clean(&crate::shape::matches(
        SLICE_SHAPE,
        &read(SLICE_SHAPE),
        &expected,
    ));
}

#[test]
fn the_milestones_shape_is_the_two_sections_under_the_h1() {
    assert_clean(&crate::shape::matches(
        MILESTONES_SHAPE,
        &read(MILESTONES_SHAPE),
        SECTIONS,
    ));
}

#[test]
fn a_local_status_heading_is_a_violation() {
    let readme = format!("{}\n## Status\n\nactive\n", healthy());
    let found = shape("r.md", &readme);
    assert!(
        found.iter().any(|v| v.to_string().contains("local status")),
        "{}",
        crate::violation::render(&found)
    );
}

#[test]
fn a_missing_slice_heading_is_a_violation() {
    let readme = healthy().replace("## Done when\n", "");
    let found = shape("r.md", &readme);
    assert_eq!(found.len(), 1, "{}", crate::violation::render(&found));
    assert!(found[0].to_string().contains("fixed slice shape"));
}

#[test]
fn an_appetite_that_disagrees_with_its_row_is_a_violation() {
    let readme = healthy();
    let found = appetite("r.md", &readme, &row("active", "4 implementation sessions"));
    assert_eq!(found.len(), 1, "{}", crate::violation::render(&found));
    assert!(found[0].to_string().contains("does not match"));
}

#[test]
fn a_governed_by_target_that_does_not_exist_is_a_violation() {
    let readme = healthy().replace("AGENTS.md](../../../../AGENTS.md", "x](./nowhere.md");
    let found = governed_by("r.md", "001-a-slice", &readme);
    assert_eq!(found.len(), 1, "{}", crate::violation::render(&found));
    assert!(found[0].to_string().contains("missing or not a file"));
}

#[test]
fn an_external_governed_by_target_is_not_resolved() {
    let readme = healthy().replace(
        "AGENTS.md](../../../../AGENTS.md",
        "spec](https://example.invalid/spec",
    );
    assert_clean(&governed_by("r.md", "001-a-slice", &readme));
}

#[test]
fn each_of_the_five_ears_shapes_is_accepted() {
    let lines = "- The wrapper shall pass argv through.\n\
        - While a verb is unimplemented, its spelling shall reach the child.\n\
        - When the child exits, the wrapper shall return its status.\n\
        - Where a profile is selected, the wrapper shall compose it.\n\
        - If the child is missing, then the wrapper shall report it.";
    assert_clean(&acceptance("r.md", &slice(lines, "- x; escape: y.")));
}

#[test]
fn a_bare_ears_trigger_without_shall_is_a_violation() {
    let readme = slice("- When the cache misses.", "- x; escape: y.");
    let found = acceptance("r.md", &readme);
    assert_eq!(found.len(), 1, "{}", crate::violation::render(&found));
    assert!(found[0].to_string().contains("not an assertion-only EARS"));
}

#[test]
fn a_when_clause_without_its_comma_is_a_violation() {
    let readme = slice(
        "- When the child exits the wrapper shall return.",
        "- x; escape: y.",
    );
    assert_eq!(acceptance("r.md", &readme).len(), 1);
}

#[test]
fn a_rabbit_hole_without_an_escape_is_a_violation() {
    let readme = slice("- The wrapper shall work.", "- Gold plating.");
    let found = rabbit_holes("r.md", &readme);
    assert_eq!(found.len(), 1, "{}", crate::violation::render(&found));
    assert!(found[0].to_string().contains("lacks escape"));
}

#[test]
fn a_healthy_tasks_file_is_accepted() {
    assert_clean(&tasks(
        "t.md",
        "# Tasks\n\n- [ ] One.\n- [x] Two.\n",
        "active",
    ));
}

#[test]
fn a_tasks_file_under_a_terminal_status_is_a_violation() {
    let found = tasks("t.md", "# Tasks\n\n- [ ] One.\n", "done");
    assert_eq!(found.len(), 1, "{}", crate::violation::render(&found));
    assert!(found[0].to_string().contains("terminal milestone status"));
}

#[test]
fn a_tasks_file_with_a_second_heading_is_a_violation() {
    let found = tasks("t.md", "# Tasks\n\n- [ ] One.\n\n## Extra\n", "active");
    assert_eq!(found.len(), 1, "{}", crate::violation::render(&found));
    assert!(found[0].to_string().contains("flat checkbox"));
}

#[test]
fn an_empty_tasks_file_is_a_violation() {
    assert_eq!(tasks("t.md", "# Tasks\n\n", "active").len(), 1);
}

#[test]
fn a_requirements_file_without_a_reasoned_gate_is_a_violation() {
    let readme = slice("", "- x; escape: y.");
    let found = requirements("r.md", &readme, "q.md");
    assert_eq!(found.len(), 1, "{}", crate::violation::render(&found));
    assert!(found[0].to_string().contains("reasoned gate"));
}

#[test]
fn a_requirements_file_with_its_gate_is_accepted() {
    let readme = slice("", "- x; escape: y.").replace(
        "## Acceptance",
        "<!-- requirements-gate: the contract is long -->\n## Acceptance",
    );
    assert_clean(&requirements("r.md", &readme, "q.md"));
}

#[test]
fn an_empty_reasoned_gate_is_a_violation() {
    let readme = slice("", "- x; escape: y.").replace(
        "## Acceptance",
        "<!-- requirements-gate: -->\n## Acceptance",
    );
    assert_eq!(requirements("r.md", &readme, "q.md").len(), 1);
}

#[test]
fn a_requirements_file_beside_a_filled_acceptance_is_a_violation() {
    let readme = healthy().replace(
        "## Acceptance",
        "<!-- requirements-gate: the contract is long -->\n## Acceptance",
    );
    let found = requirements("r.md", &readme, "q.md");
    assert_eq!(found.len(), 1, "{}", crate::violation::render(&found));
    assert!(found[0].to_string().contains("must be empty"));
}

const QUESTION: &str = "# Open questions

## Q-002 — Does it work?

Blocks: slice 003 acceptance.

Raised: while shaping.

Exit: measurement, run it and record the result.
";

#[test]
fn a_healthy_question_register_is_accepted() {
    assert_clean(&questions(QUESTION));
}

#[test]
fn a_mistyped_question_heading_is_a_violation() {
    let found = questions(&QUESTION.replace("## Q-002", "## Q2"));
    assert!(
        found
            .iter()
            .any(|v| v.to_string().contains("stable Q-NNN form")),
        "{}",
        crate::violation::render(&found)
    );
}

#[test]
fn a_reused_question_id_is_a_violation() {
    let found = questions(&format!(
        "{QUESTION}{}",
        QUESTION.replace("# Open questions\n", "")
    ));
    assert!(
        found.iter().any(|v| v.to_string().contains("is reused")),
        "{}",
        crate::violation::render(&found)
    );
}

#[test]
fn a_question_field_outside_a_block_is_a_violation() {
    let found = questions("# Open questions\n\nBlocks: nothing at all.\n");
    assert_eq!(found.len(), 1, "{}", crate::violation::render(&found));
    assert!(
        found[0]
            .to_string()
            .contains("belongs to no question block")
    );
}

#[test]
fn a_question_missing_a_field_is_a_violation() {
    let found = questions(&QUESTION.replace("Raised: while shaping.\n\n", ""));
    assert_eq!(found.len(), 1, "{}", crate::violation::render(&found));
    assert!(found[0].to_string().contains("exactly one Blocks"));
}

#[test]
fn an_exit_outside_the_three_kinds_is_a_violation() {
    let found = questions(&QUESTION.replace("Exit: measurement,", "Exit: someday,"));
    assert_eq!(found.len(), 1, "{}", crate::violation::render(&found));
    assert!(found[0].to_string().contains("permitted Exit"));
}

#[test]
fn a_question_block_ends_at_an_ordinary_heading() {
    let text = format!("{QUESTION}\n## Notes\n\nBlocks: stray.\n");
    let found = questions(&text);
    assert_eq!(found.len(), 1, "{}", crate::violation::render(&found));
    assert!(
        found[0]
            .to_string()
            .contains("belongs to no question block")
    );
}
