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
    ui::prose::{BODY_INDENT, Palette, paragraph, plural, wrap},
};

/// The one-sentence explanation of every ground a judgment can stand on.
///
/// It opens with the consequence — kept or collectable — because that is the
/// half a reader acts on, and the reason follows it.
const fn because(ground: Ground) -> &'static str {
    match ground {
        Ground::DevicePresent => "Kept: that terminal is still open.",
        Ground::DeviceAbsent => {
            "Collectable: that terminal is gone, so nothing will write here again."
        }
        Ground::DeviceUnobservable => {
            "Kept: this run could not check whether that terminal still exists."
        }
        Ground::LeaderRunning => "Kept: the process group it belongs to is still running.",
        Ground::LeaderGone => "Collectable: the process group it belonged to has exited.",
        Ground::LeaderForeignBoot => {
            "Collectable: it belongs to a boot that has ended, and no process outlives its kernel."
        }
        Ground::LeaderUnreadable => {
            "Kept: this run could not read enough about that process group to decide."
        }
        Ground::Unrecorded => concat!(
            "Kept: it carries no record of the terminal it belongs to, so nothing ",
            "about it can be proven. Its next launch writes one."
        ),
        Ground::Unplaced => {
            "Kept: this run cannot tell which namespace it is in, so it can decide nothing here."
        }
        Ground::Foreign => concat!(
            "Kept: it belongs to another machine or container sharing this storage, ",
            "and only that side can judge it."
        ),
        Ground::Alias => concat!(
            "Kept: an earlier version could not tell terminals apart and gave every ",
            "one of them this single directory, so nothing about it can be proven. ",
            "What is inside belongs to all of them."
        ),
    }
}

/// Maps a verdict onto the status colour its row borrows.
///
/// Live passes, dead is the row with something to do about it, and undecidable
/// is dimmed like a skip. The words stay the verdicts' own; only the colour is
/// shared, and stripping it changes nothing.
const fn shade(verdict: Verdict) -> CheckStatus {
    match verdict {
        Verdict::Live => CheckStatus::Pass,
        Verdict::Dead => CheckStatus::Warn,
        Verdict::Unknown => CheckStatus::Skipped,
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
    // The directory is named only where the reader is the one who would act on
    // it, and on a line of its own so it can be copied back into a shell. A
    // live session needs nothing, and a dead one is `clean`'s to remove and is
    // listed again at its prompt; an undecidable one is nobody's but the
    // reader's, so they are told where it is.
    if finding.verdict == Verdict::Unknown {
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
        "Every terminal that launches claude gets a state directory of its own. ",
        "Here is each one, and whether the terminal it belongs to still exists."
    )));
    out.push('\n');
    for finding in findings {
        out.push_str(&row(palette, finding));
    }
    out.push('\n');
    let dead = findings
        .iter()
        .filter(|finding| finding.verdict == Verdict::Dead)
        .count();
    out.push_str(&paragraph(&if dead > 0 {
        format!(
            "{dead} {} collectable. Remove {} with: claude-session-rs session clean",
            plural(dead, "session is", "sessions are"),
            plural(dead, "it", "them"),
        )
    } else {
        concat!(
            "Nothing here is collectable: every session above is either live or ",
            "undecidable, and only a terminal proven gone is ever removed."
        )
        .to_owned()
    }));
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
        return paragraph(concat!(
            "Nothing to collect. A session directory is removed only once the terminal it ",
            "belongs to is proven gone, and none here is. See what each one is judged as, ",
            "and why, with: claude-session-rs session list"
        ));
    }
    let mut rows = String::new();
    for finding in removed {
        rows.push_str(&wrap(&finding.path.display().to_string(), "  ", "      "));
    }
    let mut out = format!("{}\n\n{rows}\n", palette.heading("Collected"));
    let mut summary = format!(
        "Removed {} dead session {}",
        removed.len(),
        plural(removed.len(), "directory", "directories")
    );
    if pruned_namespaces > 0 {
        summary.push_str(&format!(
            ", and {pruned_namespaces} emptied namespace {}",
            plural(pruned_namespaces, "directory", "directories")
        ));
    }
    summary.push_str(
        ". Your login, your projects tree, and every live or undecidable session are untouched.",
    );
    out.push_str(&paragraph(&summary));
    out
}
