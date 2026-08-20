//! The session reports a person reads.
//!
//! Every fact here is also in the `--json` document beside it, which is where
//! a script was always meant to look ([ADR-0093]). What this side adds is the
//! sentence that says what a verdict means and what to type next.
//!
//! [ADR-0093]: ../../../docs/decisions/ADR-0093-write-every-non-machine-surface-for-a-person.md

#![allow(
    clippy::format_push_string,
    reason = "the renderer assembles one deterministic document before its sole write"
)]

use crate::{
    domain::witness::Verdict,
    services::session::gc::SessionFinding,
    ui::prose::{Palette, paragraph, plural, wrap},
};

/// What one verdict means, as the clause a row ends with.
const fn meaning(verdict: Verdict) -> &'static str {
    match verdict {
        Verdict::Live => "live; its terminal still exists",
        Verdict::Dead => "dead; its terminal is provably gone",
        Verdict::Unknown => "unknown; this run cannot decide, so it is kept",
    }
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
    let mut rows = String::new();
    for finding in findings {
        let text = format!(
            "{}/{}/{} — {}",
            finding.account.as_str(),
            finding.namespace.as_str(),
            finding.terminal.as_str(),
            meaning(finding.verdict)
        );
        rows.push_str(&wrap(&text, "  ", "      "));
    }
    let mut out = format!("{}\n\n{rows}", palette.heading("Sessions"));
    let dead = findings
        .iter()
        .filter(|finding| matches!(finding.verdict, Verdict::Dead))
        .count();
    if dead > 0 {
        out.push('\n');
        out.push_str(&paragraph(&format!(
            "{dead} {} dead. Collect {} with: claude-session-rs session clean",
            plural(dead, "session is", "sessions are"),
            plural(dead, "it", "them"),
        )));
    }
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
        return paragraph("Nothing was removed.");
    }
    if removed.is_empty() {
        return paragraph(concat!(
            "No session is provably dead, so there is nothing to collect. ",
            "See the verdicts with: claude-session-rs session list"
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
    summary.push('.');
    out.push_str(&paragraph(&summary));
    out
}
