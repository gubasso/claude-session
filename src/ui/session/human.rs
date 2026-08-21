//! The session reports a person reads.
//!
//! Every fact here is also in the `--json` document beside it, which is where
//! a script was always meant to look ([ADR-0093]). What this side adds is the
//! sentence that says what a verdict means and what to type next.
//!
//! A row is one bracketed verdict, the terminal it is about, and one sentence
//! of why — because a reader asking `session list` is asking whether their own
//! panes are accounted for, and a verdict with no ground behind it cannot
//! answer that.
//!
//! [ADR-0093]: ../../../docs/decisions/ADR-0093-write-every-non-machine-surface-for-a-person.md

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
    ui::prose::{BODY_INDENT, INDENT, Palette, paragraph, plural, wrap},
};

/// A paragraph and the command it points at, the command on its own line.
///
/// [`wrap`] refuses to break a path because a path split across two lines
/// cannot be copied back into a shell; a command is several words, so the wrap
/// cannot protect it and the layout has to.
fn next_command(text: &str, command: &str) -> String {
    format!("{}\n{}", paragraph(text), wrap(command, INDENT, INDENT))
}

/// The one-sentence explanation of every ground a judgment can stand on.
///
/// It opens with the consequence — kept, collectable, or refused — because
/// that is the half a reader acts on, and the reason follows it. Only a
/// terminal this run can see keeps its directory; every other ground says why
/// the run could not account for it, which is what the reader is deciding on
/// when the prompt asks.
const fn because(ground: Ground) -> &'static str {
    match ground {
        Ground::DevicePresent => {
            "Kept: that terminal is still open, whether or not claude is running in it."
        }
        Ground::DeviceAbsent => {
            "Collectable: that terminal is gone, so nothing will write here again."
        }
        Ground::DeviceUnobservable => concat!(
            "Collectable: this run could not check whether that terminal still ",
            "exists, so it cannot account for this directory."
        ),
        Ground::LeaderRunning => "Kept: the process group it belongs to is still running.",
        Ground::LeaderGone => "Collectable: the process group it belonged to has exited.",
        Ground::LeaderForeignBoot => {
            "Collectable: it belongs to a boot that has ended, and no process outlives its kernel."
        }
        Ground::LeaderUnreadable => concat!(
            "Collectable: this run could not read enough about that process group ",
            "to account for this directory."
        ),
        Ground::Unrecorded => concat!(
            "Collectable: it carries no record of the terminal it belongs to, so ",
            "nothing here can say what it is for."
        ),
        // The one ground that stops the verb rather than feeding it: a run
        // that cannot place itself would find every directory unaccounted for
        // and take the whole tree.
        Ground::Unplaced => concat!(
            "Refused: this run cannot tell which namespace it is in, so it can ",
            "judge nothing here and session clean will not act."
        ),
        Ground::Foreign => concat!(
            "Collectable: it belongs to another machine or container sharing this ",
            "storage, so nothing this run can see accounts for it."
        ),
        Ground::Alias => concat!(
            "Collectable: no terminal ever owned it. An earlier version gave every ",
            "pane of one namespace this single directory, so what is inside belongs ",
            "to all of them, and no name this version issues can claim it again."
        ),
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

/// Names the subject of one row: the terminal, and whose session it is.
///
/// A finding that names no terminal — no record at all, or one naming the
/// alias every process shares — says the account and stops, because printing
/// a name that stands for no pane is what made the old report unreadable in
/// the first place. Which findings those are is the survey's judgment, so
/// both sides of the report suppress the same ones.
fn subject(finding: &SessionFinding) -> String {
    let account = format!("account {}", finding.account.as_str());
    let mut text = match &finding.named {
        Some(named) => format!("{named}, {account}"),
        None => account,
    };
    if finding.current {
        text.push_str(" — this terminal");
    }
    text
}

/// Renders one finding as its verdict row and the sentence under it.
fn row(palette: Palette, finding: &SessionFinding) -> String {
    // Laid out with the plain token and coloured afterwards, the discipline
    // every renderer here follows: wrapping on an escaped token would let
    // colour move a line break.
    let token = format!("[{}]", finding.verdict.as_str());
    let pad = " ".repeat(BODY_INDENT.saturating_sub(2 + token.chars().count()));
    let prefix = format!("  {token}{pad}");
    let body = " ".repeat(BODY_INDENT);
    let mut text = wrap(&subject(finding), &prefix, &body);
    text.push_str(&wrap(because(finding.ground), &body, &body));
    // The directory is named on every row the reader might act on, and on a
    // line of its own so it can be copied back into a shell. A live session is
    // identified by its own device and needs nothing; a collectable one may
    // name no terminal at all, in which case the path is the only thing
    // telling one row from another.
    if finding.verdict.collectable() {
        text.push_str(&wrap(&finding.path.display().to_string(), &body, &body));
    }
    palette.recolour_token(text, &token, shade(finding.verdict))
}

/// Renders every session directory and its verdict.
pub(super) fn list(palette: Palette, findings: &[SessionFinding]) -> String {
    if findings.is_empty() {
        return format!(
            "{}\n\n{}",
            palette.heading("Sessions"),
            paragraph(concat!(
                "No session directories are stored on this machine yet. ",
                "A launch creates one per terminal."
            ))
        );
    }
    let mut out = format!("{}\n\n", palette.heading("Sessions"));
    out.push_str(&paragraph(concat!(
        "Every terminal that launches claude gets a state directory of its own, ",
        "and that directory outlives the child that made it. Here is each one, ",
        "and whether the terminal it belongs to still exists."
    )));
    out.push('\n');
    for finding in findings {
        out.push_str(&row(palette, finding));
    }
    out.push('\n');
    // A tree this run cannot place is reported and left alone: `clean` refuses
    // on it rather than mistaking its own blindness for an empty tree, so the
    // summary must not offer the verb.
    if findings
        .iter()
        .any(|finding| finding.ground == Ground::Unplaced)
    {
        out.push_str(&paragraph(concat!(
            "This run cannot tell which namespace it is in, so it can prove nothing ",
            "about the sessions above. Collection refuses here rather than emptying ",
            "a tree it cannot see."
        )));
        return out;
    }
    let collectable = findings
        .iter()
        .filter(|finding| finding.verdict.collectable())
        .count();
    if collectable > 0 {
        out.push_str(&next_command(
            &format!(
                "{collectable} {} collectable. Remove {} with:",
                plural(collectable, "session is", "sessions are"),
                plural(collectable, "it", "them"),
            ),
            "claude-session-rs session clean",
        ));
    } else {
        out.push_str(&paragraph(concat!(
            "Nothing here is collectable: every session above belongs to a terminal ",
            "this run can still see."
        )));
    }
    out
}

/// The groups a prompt separates, each with the clause that says what it is.
///
/// One entry per collectable verdict, so a verdict cannot be added without
/// stating what a reader would be losing by collecting it.
const GROUPS: &[(Verdict, &str)] = &[
    (Verdict::Dead, "belongs to a terminal that has closed"),
    (
        Verdict::Orphaned,
        concat!(
            "was never owned by any one terminal: an earlier version gave every pane ",
            "of one namespace a single shared directory, and no launch can claim it again"
        ),
    ),
    (
        Verdict::Unknown,
        concat!(
            "cannot be accounted for by this run: no readable record of the terminal ",
            "it belongs to, or a namespace this kernel cannot see"
        ),
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
            "Removing them deletes the child state and history stored there. Your ",
            "login, your projects tree, and every session belonging to a terminal ",
            "this run can still see stay."
        ),
        "",
        "",
    ));
    out.push_str("Remove them? [y/N] ");
    out
}

/// Renders what one `clean` removed, or that nothing was.
pub(super) fn collection(
    palette: Palette,
    removed: &[SessionFinding],
    pruned_namespaces: usize,
    declined: bool,
) -> String {
    if declined {
        return paragraph("Nothing was removed. Every session is exactly as it was.");
    }
    if removed.is_empty() {
        return next_command(
            concat!(
                "Nothing to collect. Every session directory here belongs to a terminal ",
                "this run can still see. See what each one is judged as, and why, with:"
            ),
            "claude-session-rs session list",
        );
    }
    let mut rows = String::new();
    for finding in removed {
        rows.push_str(&wrap(&finding.path.display().to_string(), "  ", "      "));
    }
    let mut out = format!("{}\n\n{rows}\n", palette.heading("Collected"));
    let mut summary = format!(
        "Removed {} session {}",
        removed.len(),
        plural(removed.len(), "directory", "directories")
    );
    if pruned_namespaces > 0 {
        summary.push_str(&format!(
            ", and {pruned_namespaces} emptied namespace {}",
            plural(pruned_namespaces, "directory", "directories")
        ));
    }
    summary.push_str(concat!(
        ". Your login, your projects tree, and every session belonging to a terminal ",
        "this run can still see are untouched."
    ));
    out.push_str(&paragraph(&summary));
    out
}
