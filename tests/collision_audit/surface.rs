//! Extracts the three tables the collision audit reads out of the CLI surface.
//! Not a Markdown renderer and not a general parser: it recognizes exactly the
//! pipe-table dialect `dprint` emits, and rejects anything else rather than
//! skipping it. Every branch here fails closed, because a docs gate that
//! silently matches nothing reports success.

use std::collections::BTreeMap;

use crate::audit::{SURFACE, Violation};

const FLAGS_HEADING: &str = "## Wrapper-owned flags";
const VERBS_HEADING: &str = "## Wrapper verbs";
const DOCTOR_FLAGS_HEADING: &str = "### Doctor flags";

const FLAGS_HEADER: &[&str] = &[
    "Flag",
    "Meaning",
    "Why the wrapper claims it",
    "Child status",
];
const VERBS_HEADER: &[&str] = &["Verb", "Purpose", "Grammar specified in"];
const OVERLAP_HEADER: &[&str] = &["Child verb", "Shared surface", "Resolution", "Reason"];
const DOCTOR_FLAGS_HEADER: &[&str] = &["Flag", "Meaning", "Child status"];

/// Floors, not exact counts, so the tables may grow. Lowering one takes a commit
/// that has to explain itself.
const FLAG_ROW_FLOOR: usize = 7;
const VERB_ROW_FLOOR: usize = 8;
const OVERLAP_ROW_FLOOR: usize = 2;

const CLAIMED_FLAG_SENTINELS: &[&str] = &[
    "--account",
    "--config",
    "--help",
    "--profile",
    "--quiet",
    "--verbose",
    "--version",
    "-V",
    "-h",
    "-q",
];
const CLAIMED_VERB_SENTINELS: &[&str] = &[
    "account",
    "completion",
    "config",
    "doctor",
    "help",
    "man",
    "profile",
    "session",
    "version",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Status {
    Free,
    Collides,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Resolution {
    Renamed,
    Composed,
}

#[derive(Debug)]
pub(crate) struct ClaimedFlag {
    pub(crate) spelling: String,
    pub(crate) line: usize,
    pub(crate) status: Status,
}

#[derive(Debug)]
pub(crate) struct ClaimedVerb {
    pub(crate) name: String,
    pub(crate) line: usize,
}

#[derive(Debug)]
pub(crate) struct Overlap {
    pub(crate) verb: String,
    pub(crate) line: usize,
    pub(crate) resolution: Resolution,
}

#[derive(Debug)]
pub(crate) struct Surface {
    pub(crate) flags: Vec<ClaimedFlag>,
    pub(crate) verbs: Vec<ClaimedVerb>,
    pub(crate) overlaps: Vec<Overlap>,
    pub(crate) doctor_flags: Vec<ClaimedFlag>,
    flag_rows: usize,
    verb_rows: usize,
    overlap_rows: usize,
    doctor_flag_rows: usize,
}

struct Row {
    line: usize,
    cells: Vec<String>,
}

struct Table {
    header: Vec<String>,
    rows: Vec<Row>,
}

pub(crate) fn parse(md: &str) -> Result<Surface, Vec<Violation>> {
    let (tables, mut found) = scan(md);

    // The verb metadata table is the sole table directly under its heading;
    // scoped doctor metadata follows under its own heading. Requiring an exact
    // count stops another table from being silently ignored, the same defect as
    // matching none.
    let under_verbs = tables
        .iter()
        .filter(|(heading, _)| heading == VERBS_HEADING)
        .count();
    if under_verbs != 1 {
        found.push(Violation::whole(
            SURFACE,
            format!("`{VERBS_HEADING}` holds {under_verbs} tables and the audit reads exactly one"),
        ));
    }

    let flags_table = pick(&tables, FLAGS_HEADING, FLAGS_HEADER, &mut found);
    let verbs_table = pick(&tables, VERBS_HEADING, VERBS_HEADER, &mut found);
    let overlap_table = pick(&tables, DOCTOR_FLAGS_HEADING, OVERLAP_HEADER, &mut found);
    let doctor_flags_table = pick(
        &tables,
        DOCTOR_FLAGS_HEADING,
        DOCTOR_FLAGS_HEADER,
        &mut found,
    );

    let (Some(flags_table), Some(verbs_table), Some(overlap_table), Some(doctor_flags_table)) =
        (flags_table, verbs_table, overlap_table, doctor_flags_table)
    else {
        return Err(found);
    };

    let mut flags = Vec::new();
    for row in &flags_table.rows {
        let spellings = match spellings(&row.cells[0], row.line, true) {
            Ok(spellings) => spellings,
            Err(violation) => {
                found.push(violation);
                continue;
            }
        };
        match statuses(&row.cells[3], row.line, &spellings) {
            Ok(assigned) => {
                for (spelling, status) in assigned {
                    flags.push(ClaimedFlag {
                        spelling,
                        line: row.line,
                        status,
                    });
                }
            }
            Err(mut violations) => found.append(&mut violations),
        }
    }

    let mut verbs = Vec::new();
    for row in &verbs_table.rows {
        match spellings(&row.cells[0], row.line, false) {
            Ok(names) => verbs.extend(names.into_iter().map(|name| ClaimedVerb {
                name,
                line: row.line,
            })),
            Err(violation) => found.push(violation),
        }
    }

    let mut overlaps = Vec::new();
    for row in &overlap_table.rows {
        let names = match spellings(&row.cells[0], row.line, false) {
            Ok(names) => names,
            Err(violation) => {
                found.push(violation);
                continue;
            }
        };
        // The closed set is contractual: the surface states these are the only
        // two resolutions available, so a third spelling is a defect, not a case
        // to pass through.
        let resolution = match row.cells[2].as_str() {
            "Renamed" => Resolution::Renamed,
            "Composed" => Resolution::Composed,
            other => {
                found.push(Violation::at(
                    SURFACE,
                    row.line,
                    format!("resolution `{other}` is neither renamed nor composed"),
                ));
                continue;
            }
        };
        overlaps.extend(names.into_iter().map(|verb| Overlap {
            verb,
            line: row.line,
            resolution,
        }));
    }

    let mut doctor_flags = Vec::new();
    for row in &doctor_flags_table.rows {
        match spellings(&row.cells[0], row.line, true).and_then(|values| {
            statuses(&row.cells[2], row.line, &values).map_err(|mut errors| errors.remove(0))
        }) {
            Ok(values) => {
                doctor_flags.extend(values.into_iter().map(|(spelling, status)| ClaimedFlag {
                    spelling,
                    line: row.line,
                    status,
                }))
            }
            Err(error) => found.push(error),
        }
    }

    if !found.is_empty() {
        return Err(found);
    }

    Ok(Surface {
        flags,
        verbs,
        overlaps,
        doctor_flags,
        flag_rows: flags_table.rows.len(),
        verb_rows: verbs_table.rows.len(),
        overlap_rows: overlap_table.rows.len(),
        doctor_flag_rows: doctor_flags_table.rows.len(),
    })
}

/// Proves the parse recovered the tables it was aiming at. Without these, an
/// empty parse and a clean audit are indistinguishable.
pub(crate) fn contracts(surface: &Surface) -> Vec<Violation> {
    let mut found = Vec::new();

    for (name, rows, floor) in [
        ("wrapper-owned flags", surface.flag_rows, FLAG_ROW_FLOOR),
        ("wrapper verbs", surface.verb_rows, VERB_ROW_FLOOR),
        ("verb overlap", surface.overlap_rows, OVERLAP_ROW_FLOOR),
        ("doctor flags", surface.doctor_flag_rows, 3),
    ] {
        if rows < floor {
            found.push(Violation::whole(
                SURFACE,
                format!(
                    "the {name} table parsed to {rows} rows and the audit expects at least {floor}"
                ),
            ));
        }
    }
    for sentinel in ["--json", "--list", "--strict"] {
        if !surface
            .doctor_flags
            .iter()
            .any(|flag| flag.spelling == sentinel)
        {
            found.push(Violation::whole(
                SURFACE,
                format!("the doctor flags table did not parse `{sentinel}`"),
            ));
        }
    }

    for sentinel in CLAIMED_FLAG_SENTINELS {
        if !surface.flags.iter().any(|f| f.spelling == *sentinel) {
            found.push(Violation::whole(
                SURFACE,
                format!("the flags table did not parse to a claim on `{sentinel}`"),
            ));
        }
    }
    for sentinel in CLAIMED_VERB_SENTINELS {
        if !surface.verbs.iter().any(|v| v.name == *sentinel) {
            found.push(Violation::whole(
                SURFACE,
                format!("the verbs table did not parse to a claim on `{sentinel}`"),
            ));
        }
    }

    if surface
        .flags
        .iter()
        .any(|f| f.spelling == "--verbose" && f.status != Status::Collides)
    {
        found.push(Violation::whole(
            SURFACE,
            "`--verbose` did not parse as a collision",
        ));
    }

    // The `--config <path>` row proves the placeholder strip works: a row that
    // yields two spellings means `<path>` was read as a second flag.
    if let Some(row) = surface.flags.iter().find(|f| f.spelling == "--config") {
        let on_row = surface.flags.iter().filter(|f| f.line == row.line).count();
        if on_row != 1 {
            found.push(Violation::at(
                SURFACE,
                row.line,
                format!(
                    "the `--config` row parsed to {on_row} spellings and it claims \
                    one; the value placeholder is being read as a flag"
                ),
            ));
        }
    }

    // The `--version`, `-V` row is the only per-spelling status in the table,
    // so it proves the semicolon split and the attribution rule both work.
    if let Some(row) = surface.flags.iter().find(|f| f.spelling == "--version") {
        let on_row: Vec<&ClaimedFlag> = surface
            .flags
            .iter()
            .filter(|f| f.line == row.line)
            .collect();
        let free = on_row.iter().filter(|f| f.status == Status::Free).count();
        let collides = on_row
            .iter()
            .filter(|f| f.status == Status::Collides)
            .count();
        if (free, collides) != (1, 1) {
            found.push(Violation::at(
                SURFACE,
                row.line,
                format!(
                    "the `--version` row parsed to {free} free and {collides} \
                    colliding spellings, and it states one of each"
                ),
            ));
        }
    }

    for (verb, resolution) in [
        ("auth", Resolution::Renamed),
        ("doctor", Resolution::Composed),
    ] {
        if !surface
            .overlaps
            .iter()
            .any(|o| o.verb == verb && o.resolution == resolution)
        {
            found.push(Violation::whole(
                SURFACE,
                format!("the overlap table did not parse to `{verb}` resolved as {resolution:?}"),
            ));
        }
    }

    found
}

fn pick<'a>(
    tables: &'a [(String, Table)],
    heading: &str,
    header: &[&str],
    found: &mut Vec<Violation>,
) -> Option<&'a Table> {
    let mut matched = tables
        .iter()
        .filter(|(h, t)| h == heading && t.header == header)
        .map(|(_, t)| t);
    let first = matched.next();
    if first.is_none() {
        found.push(Violation::whole(
            SURFACE,
            format!("no table under `{heading}` has the header {header:?}"),
        ));
    } else if matched.next().is_some() {
        found.push(Violation::whole(
            SURFACE,
            format!("more than one table under `{heading}` has the header {header:?}"),
        ));
        return None;
    }
    first
}

fn scan(md: &str) -> (Vec<(String, Table)>, Vec<Violation>) {
    let lines: Vec<&str> = md.lines().collect();
    let mut tables = Vec::new();
    let mut found = Vec::new();
    let mut heading = String::new();
    let mut fenced = false;
    let mut i = 0;

    while i < lines.len() {
        let trimmed = lines[i].trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            fenced = !fenced;
            i += 1;
            continue;
        }
        if fenced {
            i += 1;
            continue;
        }
        if trimmed.starts_with('#') {
            heading = lines[i].trim().to_string();
            i += 1;
            continue;
        }
        // A header line alone is not enough: requiring the delimiter row too is
        // what keeps a prose sentence containing a pipe from becoming a table.
        if !(trimmed.starts_with('|') && lines.get(i + 1).is_some_and(|l| is_delimiter(l))) {
            i += 1;
            continue;
        }

        let header = cells(lines[i]);
        let mut rows = Vec::new();
        let mut j = i + 2;
        while j < lines.len() && lines[j].trim_start().starts_with('|') {
            let cells = cells(lines[j]);
            if cells.len() == header.len() {
                rows.push(Row { line: j + 1, cells });
            } else {
                found.push(Violation::at(
                    SURFACE,
                    j + 1,
                    format!(
                        "this row has {} cells and its header has {}; an unescaped \
                        `|` inside a cell shifts every column after it",
                        cells.len(),
                        header.len()
                    ),
                ));
            }
            j += 1;
        }
        tables.push((heading.clone(), Table { header, rows }));
        i = j;
    }

    (tables, found)
}

fn is_delimiter(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with('|')
        && trimmed.contains('-')
        && trimmed.chars().all(|c| matches!(c, '|' | '-' | ':' | ' '))
}

fn cells(line: &str) -> Vec<String> {
    let trimmed = line.trim();
    let trimmed = trimmed.strip_prefix('|').unwrap_or(trimmed);
    let trimmed = trimmed.strip_suffix('|').unwrap_or(trimmed);
    trimmed
        .split('|')
        .map(|cell| cell.trim().to_string())
        .collect()
}

/// Backtick spans, not a comma split: the same primitive reads a one-spelling
/// cell, a two-spelling cell, and a cell carrying a value placeholder.
fn spellings(cell: &str, line: usize, want_flag: bool) -> Result<Vec<String>, Violation> {
    let spans = backticked(cell);
    if spans.is_empty() {
        return Err(Violation::at(
            SURFACE,
            line,
            "the first column names no backticked spelling",
        ));
    }
    let mut out = Vec::new();
    for span in spans {
        let spelling = span
            .split_once(" <")
            .map_or(span.as_str(), |(name, _)| name)
            .trim();
        if spelling.is_empty() || spelling.contains(char::is_whitespace) {
            return Err(Violation::at(
                SURFACE,
                line,
                format!("`{span}` is not a single spelling"),
            ));
        }
        if spelling.starts_with('-') != want_flag {
            let expected = if want_flag { "a flag" } else { "a verb" };
            return Err(Violation::at(
                SURFACE,
                line,
                format!("`{spelling}` is not {expected} spelling"),
            ));
        }
        out.push(spelling.to_string());
    }
    Ok(out)
}

/// A clause naming spellings applies to those; a clause naming none applies to
/// every spelling in the row. An unrecognized clause is a failure and never a
/// default, because defaulting to free is exactly the collision this gate exists
/// to catch.
///
/// The grammar is closed and matched whole, in the same way the resolution
/// column is. A substring test would read `Does not collide` as a collision —
/// the exact inversion of what its author wrote — and a row whose spelling the
/// child does happen to own would then stay green on a claim that says the
/// opposite.
fn status_of(clause: &str) -> Option<Status> {
    let normalized = without_backticked(clause).trim().to_ascii_lowercase();
    match normalized.as_str() {
        "free" => Some(Status::Free),
        "collides by design" => Some(Status::Collides),
        // The prose after the colon explains the collision and is not graded.
        rest if rest.starts_with("collides:") => Some(Status::Collides),
        _ => None,
    }
}

fn statuses(
    cell: &str,
    line: usize,
    spellings: &[String],
) -> Result<Vec<(String, Status)>, Vec<Violation>> {
    let mut assigned: BTreeMap<String, Status> = BTreeMap::new();
    let mut found = Vec::new();

    for clause in cell.split(';') {
        let Some(status) = status_of(clause) else {
            found.push(Violation::at(
                SURFACE,
                line,
                format!(
                    "unrecognized child-status clause \"{}\"; apart from any \
                    backticked spellings a clause reads exactly `free`, exactly \
                    `collides by design`, or begins `collides:` and then explains \
                    itself",
                    clause.trim()
                ),
            ));
            continue;
        };

        let named = backticked(clause);
        let targets = if named.is_empty() {
            spellings.to_vec()
        } else {
            named
        };
        for target in targets {
            if !spellings.contains(&target) {
                found.push(Violation::at(
                    SURFACE,
                    line,
                    format!(
                        "the child status column names `{target}` and this row does not claim it"
                    ),
                ));
            } else if assigned.insert(target.clone(), status).is_some() {
                found.push(Violation::at(
                    SURFACE,
                    line,
                    format!("`{target}` is given two child statuses on one row"),
                ));
            }
        }
    }

    for spelling in spellings {
        if !assigned.contains_key(spelling) {
            found.push(Violation::at(
                SURFACE,
                line,
                format!(
                    "`{spelling}` is claimed on this row and the child status \
                    column says nothing about it"
                ),
            ));
        }
    }

    if found.is_empty() {
        Ok(assigned.into_iter().collect())
    } else {
        Err(found)
    }
}

fn backticked(cell: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = cell;
    while let Some(open) = rest.find('`') {
        let after = &rest[open + 1..];
        let Some(close) = after.find('`') else { break };
        out.push(after[..close].to_string());
        rest = &after[close + 1..];
    }
    out
}

fn without_backticked(cell: &str) -> String {
    let mut out = String::new();
    let mut rest = cell;
    while let Some(open) = rest.find('`') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        match after.find('`') {
            Some(close) => rest = &after[close + 1..],
            None => {
                rest = "";
                break;
            }
        }
    }
    out.push_str(rest);
    out
}
