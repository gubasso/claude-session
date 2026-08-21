//! The session reports a person reads.
//!
//! Every fact here is also in the `--json` document beside it, which is where
//! a script was always meant to look ([ADR-0093]).
//!
//! A list is read by scanning, not by reading sentences, so this renders one
//! table and nothing else: the verdict, the session, whose account it is, and
//! the short reason behind the verdict. The long form of any reason lives in
//! [sessions]; a report that explained itself on every row would bury the rows
//! it exists to show.
//!
//! A session is named as its reader knows it — the name the child registered,
//! and the directory only where there is no such name ([ADR-0114]). The
//! directory is on every row of the document beside this one, which is where a
//! caller that wants the wrapper's own identifier was always meant to look.
//!
//! [ADR-0093]: ../../../docs/decisions/ADR-0093-write-every-non-machine-surface-for-a-person.md
//! [ADR-0114]: ../../../docs/decisions/ADR-0114-name-a-reported-session-as-the-child-does.md
//! [sessions]: ../../../docs/reference/sessions.md

#![allow(
    clippy::format_push_string,
    reason = "the renderer assembles one deterministic document before its sole write"
)]

use crate::{
    domain::{
        checks::CheckStatus,
        witness::{Ground, Verdict},
    },
    services::session::gc::SessionFinding,
    ui::prose::{INDENT, Palette, paragraph, plural, table, wrap},
};

/// The short reason a row carries, in the column beside it.
///
/// A phrase rather than a sentence: it sits in a column, and what a reader
/// does about it is the same for every collectable row.
const fn because(ground: Ground) -> &'static str {
    match ground {
        Ground::Running => "running",
        Ground::Gone => "agent exited",
        Ground::ForeignBoot => "from an earlier boot",
        Ground::Unrecorded => "no record of what it was",
        Ground::Foreign => "another machine's",
        Ground::Unreadable => "start time unreadable",
        Ground::Unplaced => "cannot be judged here",
    }
}

/// Maps a verdict onto the status colour its row borrows.
///
/// Live passes and everything else is a row with something to do about it, so
/// the colour tracks collectability rather than the particular word. The words
/// stay the verdicts' own; only the colour is shared, and stripping it changes
/// nothing.
const fn shade(verdict: Verdict) -> CheckStatus {
    if verdict.collectable() {
        CheckStatus::Warn
    } else {
        CheckStatus::Pass
    }
}

/// The columns a row is read across, in scanning order.
///
/// The verdict first, because it is what a reader is scanning for; the session
/// next, because it is what they act on; the account and the reason after,
/// because they qualify a row already found.
const COLUMNS: &[&str] = &["status", "session", "account", "why"];

/// Names one session the way its reader does.
///
/// The child's name when a registration proved to be that agent's, and the
/// directory otherwise. The fallback is not a placeholder: an unnamed session
/// is one no reachable registration accounts for, and its directory is the
/// only name anybody has for it ([ADR-0114]).
///
/// [ADR-0114]: ../../../docs/decisions/ADR-0114-name-a-reported-session-as-the-child-does.md
fn subject(finding: &SessionFinding) -> &str {
    finding
        .name
        .as_deref()
        .unwrap_or_else(|| finding.session.as_str())
}

/// The reason column: the ground as a phrase, and the reader's own row marked.
fn why(finding: &SessionFinding) -> String {
    let mut reason = because(finding.ground).to_owned();
    if finding.current {
        reason.push_str(", this session");
    }
    reason
}

/// Renders every session directory and its verdict.
pub(super) fn list(palette: Palette, findings: &[SessionFinding]) -> String {
    if findings.is_empty() {
        return format!(
            "{}\n\n{}",
            palette.heading("Sessions"),
            paragraph(concat!(
                "No sessions. Launching claude creates one, and it lasts as long ",
                "as that claude does."
            ))
        );
    }
    let cells: Vec<Vec<String>> = findings
        .iter()
        .map(|finding| {
            vec![
                format!("[{}]", finding.verdict.as_str()),
                subject(finding).to_owned(),
                finding.account.as_str().to_owned(),
                why(finding),
            ]
        })
        .collect();
    let (header, rows) = table(COLUMNS, &cells);
    let mut out = format!("{}\n\n{header}", palette.heading("Sessions"));
    for (finding, row) in findings.iter().zip(rows) {
        // Laid out plain, then coloured, so colour cannot move a column: the
        // token the palette replaces is the one already sitting in the row.
        let token = format!("[{}]", finding.verdict.as_str());
        out.push_str(&palette.recolour_token(format!("{row}\n"), &token, shade(finding.verdict)));
    }
    out.push('\n');
    out.push_str(&summary(findings));
    out
}

/// The one line under the rows: what the verb would take, and the verb.
///
/// Short enough not to wrap, because a command broken across two lines cannot
/// be copied back into a shell.
fn summary(findings: &[SessionFinding]) -> String {
    if findings
        .iter()
        .any(|finding| finding.ground == Ground::Unplaced)
    {
        return paragraph(concat!(
            "This run cannot name its own namespace or boot, so it can prove nothing ",
            "above. Collection refuses rather than emptying a tree it cannot see."
        ));
    }
    let collectable = findings
        .iter()
        .filter(|finding| finding.verdict.collectable())
        .count();
    if collectable == 0 {
        return paragraph("Nothing to collect: every session above is still running.");
    }
    paragraph(&format!(
        "{collectable} of {} collectable. Remove {}: claude-session session clean",
        findings.len(),
        plural(collectable, "it with", "them with"),
    ))
}

/// The groups a prompt separates, each with the clause that says what it is.
///
/// One entry per collectable verdict, so a verdict cannot be added without
/// stating what a reader would be losing by collecting it.
const GROUPS: &[(Verdict, &str)] = &[
    (Verdict::Dead, "whose agent has exited"),
    (
        Verdict::Unknown,
        "this run cannot account for: no readable record, or a namespace this kernel cannot see",
    ),
];

/// The one question `session clean` asks, and what answering it costs.
///
/// Grouped rather than listed flat, because the groups lose different things
/// and a reader answers one question about all of them. The preview is part of
/// the question, so every path the verb would delete is named before it asks.
pub(super) fn prompt(collectable: &[SessionFinding]) -> String {
    let total = collectable.len();
    let mut out = format!(
        "{total} session {} collectable.\n\n",
        plural(total, "directory is", "directories are")
    );
    for (verdict, clause) in GROUPS {
        let group: Vec<&SessionFinding> = collectable
            .iter()
            .filter(|finding| finding.verdict == *verdict)
            .collect();
        if group.is_empty() {
            continue;
        }
        out.push_str(&wrap(&format!("{} {clause}:", group.len()), "", ""));
        for finding in group {
            out.push_str(&wrap(&finding.path.display().to_string(), "  ", "      "));
        }
        out.push('\n');
    }
    out.push_str(&wrap(
        concat!(
            "Removing them deletes the child state and history stored there. Your login, ",
            "your projects tree, and every running session stay."
        ),
        "",
        "",
    ));
    out.push_str("Remove them? [y/N] ");
    out
}

/// Renders what one collection removed, or that nothing was.
pub(super) fn collection(
    palette: Palette,
    removed: &[SessionFinding],
    pruned_namespaces: usize,
    declined: bool,
) -> String {
    if declined {
        return paragraph("Nothing was removed.");
    }
    if removed.is_empty() {
        return paragraph("Nothing to collect: every session is still running.");
    }
    let mut out = format!("{}\n\n", palette.heading("Collected"));
    for finding in removed {
        out.push_str(&wrap(&finding.path.display().to_string(), INDENT, "      "));
    }
    out.push('\n');
    let mut summary = format!(
        "Removed {} session {}",
        removed.len(),
        plural(removed.len(), "directory", "directories")
    );
    if pruned_namespaces > 0 {
        summary.push_str(&format!(
            " and {pruned_namespaces} emptied namespace {}",
            plural(pruned_namespaces, "directory", "directories")
        ));
    }
    summary.push_str(". Running sessions, your login, and your projects are untouched.");
    out.push_str(&paragraph(&summary));
    out
}
