//! Deterministic human and JSON projections of the doctor catalog.
//!
//! The two projections have two audiences and one source. The human one is
//! written for a person: titles, consequences, and a next action, and no
//! identifier, count, or exit code a reader did not ask for. Every field it
//! leaves out is in the machine document below, which is where a script was
//! always meant to read ([ADR-0093]).
//!
//! [ADR-0093]: ../../docs/decisions/ADR-0093-write-every-non-machine-surface-for-a-person.md

#![allow(
    clippy::format_collect,
    clippy::format_push_string,
    reason = "the renderer assembles one deterministic document before its sole write"
)]

use serde_json::{Value, json};

use crate::{
    domain::checks::{
        CATALOG, Check, CheckResult, CheckStatus, ChildReport, ChildStatus, Scope, Severity,
        Summary, Verdict,
    },
    error::{AppError, Diagnostic, ErrorKind},
    ui::writer::{Color, OutputWriter},
};

/// The column prose starts at: two spaces, the widest status word, two spaces.
const BODY_INDENT: usize = 13;
/// The column every line wraps at. A constant rather than the terminal width,
/// so the bytes into a pipe are the bytes into a terminal.
const WRAP_AT: usize = 76;

fn json_error(error: &serde_json::Error) -> AppError {
    AppError::new(
        ErrorKind::Internal,
        Diagnostic::new(
            "doctor JSON failed",
            "doctor document",
            error.to_string(),
            "report this wrapper bug",
        ),
    )
}

/// Wraps `text` at [`WRAP_AT`], prefixing the first line and the rest apart.
///
/// A word longer than the remaining room overflows rather than being broken:
/// the long words here are filesystem paths, and a path split across two lines
/// cannot be copied back into a shell.
fn wrap(text: &str, first_prefix: &str, continuation: &str) -> String {
    let mut out = String::new();
    let mut line = first_prefix.to_owned();
    let mut empty = true;
    for word in text.split_whitespace() {
        if !empty && line.chars().count() + 1 + word.chars().count() > WRAP_AT {
            out.push_str(line.trim_end());
            out.push('\n');
            continuation.clone_into(&mut line);
            empty = true;
        }
        if !empty {
            line.push(' ');
        }
        line.push_str(word);
        empty = false;
    }
    if !empty {
        out.push_str(line.trim_end());
        out.push('\n');
    }
    out
}

/// The four-bit escapes for the two coloured `doctor` surfaces.
///
/// Nothing here carries meaning: the status word already spells the status and
/// the heading already spells the scope, which is the condition the closed
/// surface set attaches to a colour ([ADR-0082]).
///
/// [ADR-0082]: ../../docs/decisions/ADR-0082-colour-a-closed-set-of-named-surfaces.md
#[derive(Clone, Copy)]
struct Palette(bool);

impl Palette {
    fn status(self, status: CheckStatus) -> String {
        let token = format!("[{}]", status.as_str());
        if !self.0 {
            return token;
        }
        let code = match status {
            CheckStatus::Pass => "32",
            CheckStatus::Warn => "33",
            CheckStatus::Fail => "31",
            CheckStatus::Skipped => "2",
        };
        format!("\u{1b}[{code}m{token}\u{1b}[0m")
    }
    fn heading(self, text: &str) -> String {
        if self.0 {
            format!("\u{1b}[1m{text}\u{1b}[0m")
        } else {
            text.to_owned()
        }
    }
}

/// Writes static catalog metadata without observing the host.
pub(crate) fn list(writer: &OutputWriter, json_mode: bool, color: Color) -> Result<(), AppError> {
    let bytes = if json_mode {
        let checks: Vec<Value> = CATALOG
            .iter()
            .map(|check| {
                json!({
                    "id": check.id(),
                    "scope": check.scope().as_str(),
                    "severity": check.severity().as_str(),
                })
            })
            .collect();
        let mut bytes = serde_json::to_vec(&json!({"schema_version": 1, "checks": checks}))
            .map_err(|error| json_error(&error))?;
        bytes.push(b'\n');
        bytes
    } else {
        let palette = Palette(color.stdout());
        let mut text = String::new();
        for scope in [Scope::Host, Scope::Session] {
            text.push_str(&palette.heading(scope_heading(scope)));
            text.push_str("\n\n");
            for check in CATALOG.iter().filter(|check| check.scope() == scope) {
                let requirement = match check.severity() {
                    Severity::Hard => "required",
                    Severity::Soft => "optional",
                };
                text.push_str(&wrap(
                    &format!("{} ({requirement}) — {}", check.title(), check.id()),
                    "  ",
                    "    ",
                ));
            }
            text.push('\n');
        }
        text.into_bytes()
    };
    writer
        .stdout(&bytes)
        .map_err(|error| super::writer::output_error(&error))
}

/// Returns the heading that says what a scope covers.
const fn scope_heading(scope: Scope) -> &'static str {
    match scope {
        Scope::Host => "Host — this machine, the wrapper's own files, and the claude program",
        Scope::Session => "Session — the account and profile a launch would use",
    }
}

/// One rendered block: a run of adjacent checks reporting the same thing.
struct Row<'a> {
    checks: Vec<Check>,
    result: &'a CheckResult,
}

impl Row<'_> {
    /// Returns the title, which is the group's once a run has collapsed.
    fn title(&self) -> &'static str {
        if self.checks.len() > 1 {
            self.checks[0]
                .group()
                .unwrap_or_else(|| self.checks[0].title())
        } else {
            self.checks[0].title()
        }
    }
}

/// Groups adjacent results that share a group name and report the same thing.
///
/// Only adjacency and identical text collapse, so catalog order is preserved
/// and no row can stand for a result it does not describe.
fn rows<'a>(results: &'a [CheckResult], scope: Scope) -> Vec<Row<'a>> {
    let mut rows: Vec<Row<'a>> = Vec::new();
    for result in results
        .iter()
        .filter(|result| result.check.scope() == scope)
    {
        let joinable = |previous: &Row<'a>| {
            previous.checks[0].group().is_some()
                && previous.checks[0].group() == result.check.group()
                && previous.result.status == result.status
                && previous.result.message == result.message
                && previous.result.hint == result.hint
                && previous.result.reason == result.reason
        };
        if let Some(previous) = rows.last_mut()
            && joinable(previous)
        {
            previous.checks.push(result.check);
            continue;
        }
        rows.push(Row {
            checks: vec![result.check],
            result,
        });
    }
    rows
}

/// Writes one complete report and its composed child section.
pub(crate) fn report(
    writer: &OutputWriter,
    json_mode: bool,
    color: Color,
    results: &[CheckResult],
    summary: Summary,
    child: &ChildReport,
    verdict: Verdict,
) -> Result<(), AppError> {
    if json_mode {
        return json_report(writer, results, summary, child, verdict);
    }
    let palette = Palette(color.stdout());
    let mut text = String::new();
    for scope in [Scope::Host, Scope::Session] {
        text.push_str(&palette.heading(scope_heading(scope)));
        text.push_str("\n\n");
        for row in rows(results, scope) {
            text.push_str(&render_row(&row, palette));
        }
        text.push('\n');
    }
    text.push_str(&palette.heading("Summary"));
    text.push_str("\n\n");
    text.push_str(&render_summary(summary, child, verdict));
    writer
        .stdout(text.as_bytes())
        .map_err(|error| super::writer::output_error(&error))?;
    // Nothing follows the child's bytes, so every wrapper line is already out.
    if let Some(output) = &child.output {
        writer
            .delimiter("doctor")
            .map_err(|error| super::writer::output_error(&error))?;
        writer
            .stdout(output)
            .map_err(|error| super::writer::output_error(&error))?;
    }
    Ok(())
}

/// Renders one row: a passing line, or a defect with its consequence and remedy.
fn render_row(row: &Row<'_>, palette: Palette) -> String {
    let result = row.result;
    // The whole row is laid out with the plain status token and coloured only
    // afterwards. Wrapping on the escaped token would let colour move a line
    // break, and rule 1 says stripping colour changes nothing.
    let token = format!("[{}]", result.status.as_str());
    let pad = " ".repeat(BODY_INDENT.saturating_sub(2 + token.chars().count()));
    let prefix = format!("  {token}{pad}");
    let body = " ".repeat(BODY_INDENT);

    let colour = |text: String| {
        if palette.0 {
            text.replacen(&token, &palette.status(result.status), 1)
        } else {
            text
        }
    };

    if result.status == CheckStatus::Pass {
        if result.message.is_empty() {
            return colour(wrap(row.title(), &prefix, &body));
        }
        // Inline while it fits, so a healthy report stays one line per check.
        // A detail that would wrap goes below instead, because a dash left
        // hanging at the end of a line reads as a missing word.
        let inline = wrap(
            &format!("{} — {}", row.title(), result.message),
            &prefix,
            &body,
        );
        if inline.lines().count() == 1 {
            return colour(inline);
        }
        let mut text = wrap(row.title(), &prefix, &body);
        text.push_str(&wrap(&result.message, &body, &body));
        return colour(text);
    }

    let mut text = wrap(row.title(), &prefix, &body);
    if result.status == CheckStatus::Skipped {
        let reason = result.reason.as_deref().unwrap_or("no reason was recorded");
        text.push_str(&wrap(&format!("Not applicable: {reason}"), &body, &body));
    } else {
        if !result.message.is_empty() {
            text.push_str(&wrap(&result.message, &body, &body));
        }
        text.push_str(&wrap(row.checks[0].consequence(), &body, &body));
        if let Some(hint) = &result.hint {
            text.push_str(&wrap(&format!("What to do: {hint}"), &body, &body));
        }
    }
    let label = if row.checks.len() > 1 {
        "checks:"
    } else {
        "check:"
    };
    let ids: Vec<&str> = row.checks.iter().map(|check| check.id()).collect();
    text.push_str(&wrap(&format!("{label} {}", ids.join(", ")), &body, &body));
    colour(text)
}

/// States the three levels in prose, and the exit only when it is not zero.
fn render_summary(summary: Summary, child: &ChildReport, verdict: Verdict) -> String {
    let body = "  ";
    let mut counts = format!("{} checks: {} passed", summary.total, summary.passed);
    if summary.warned > 0 {
        counts.push_str(&format!(
            ", {} {}",
            summary.warned,
            plural(summary.warned, "warning", "warnings")
        ));
    }
    if summary.failed > 0 {
        counts.push_str(&format!(
            ", {} {}",
            summary.failed,
            plural(summary.failed, "failure", "failures")
        ));
    }
    if summary.skipped > 0 {
        counts.push_str(&format!(", {} not applicable", summary.skipped));
    }
    counts.push('.');
    let mut text = wrap(&counts, body, body);

    let wrapper_line = if summary.hard_failures > 0 {
        "The wrapper cannot run until the failures above are fixed."
    } else if summary.warned > 1 {
        "Nothing is blocking a launch; the warnings above are worth reading."
    } else if summary.warned == 1 {
        "Nothing is blocking a launch; the warning above is worth reading."
    } else {
        "Everything the wrapper needs is in place."
    };
    text.push_str(&wrap(wrapper_line, body, body));

    let child_line = match child.status {
        ChildStatus::Pass => "claude's own checkup reported no problems.".to_owned(),
        ChildStatus::Fail => child.exit.map_or_else(
            || {
                format!(
                    "claude's own checkup did not finish: {}.",
                    child.reason.as_deref().unwrap_or("no reason was recorded")
                )
            },
            |exit| format!("claude's own checkup reported a problem, and exited {exit}."),
        ),
        ChildStatus::Skipped => format!(
            "claude's own checkup did not run: {}.",
            child.reason.as_deref().unwrap_or("no reason was recorded")
        ),
    };
    text.push_str(&wrap(&child_line, body, body));

    // Which level produced the code, so the status is never explained by a
    // value the report omits — the property the three levels carried as fields.
    if verdict.exit != 0 {
        let source = if summary.exit != 0 {
            "from the failed checks above"
        } else if child.status == ChildStatus::Fail {
            "because claude's own checkup failed"
        } else {
            "because --strict treats a warning as a failure"
        };
        text.push_str(&wrap(
            &format!("This run exits {} {source}.", verdict.exit),
            body,
            body,
        ));
    }
    text
}

const fn plural(count: usize, one: &'static str, many: &'static str) -> &'static str {
    if count == 1 { one } else { many }
}

fn json_report(
    writer: &OutputWriter,
    results: &[CheckResult],
    summary: Summary,
    child: &ChildReport,
    verdict: Verdict,
) -> Result<(), AppError> {
    let checks: Vec<Value> = results
        .iter()
        .map(|result| {
            let mut value = json!({
                "id": result.check.id(),
                "scope": result.check.scope().as_str(),
                "severity": result.check.severity().as_str(),
                "status": result.status.as_str(),
                "message": result.message,
            });
            if matches!(result.status, CheckStatus::Warn | CheckStatus::Fail) {
                if let Some(kind) = result.kind() {
                    value["kind"] = json!(kind.spelling());
                }
                if let Some(hint) = &result.hint {
                    value["hint"] = json!(hint);
                }
            }
            if result.status == CheckStatus::Skipped {
                value["reason"] = json!(result.reason);
            }
            value
        })
        .collect();
    let mut child_value = json!({"status": child.status.as_str()});
    if let Some(output) = &child.output {
        child_value["output"] = json!(String::from_utf8_lossy(output));
    }
    if let Some(exit) = child.exit {
        child_value["exit"] = json!(exit);
    }
    if let Some(reason) = &child.reason {
        child_value["reason"] = json!(reason);
    }
    let wrapper_value = json!({
        "status": summary.status().as_str(),
        "checks": checks,
        "summary": {
            "total": summary.total,
            "passed": summary.passed,
            "warned": summary.warned,
            "failed": summary.failed,
            "skipped": summary.skipped,
            "hard_failures": summary.hard_failures,
            "exit": summary.exit,
        },
    });
    let document = json!({
        "schema_version": 1,
        "status": verdict.status.as_str(),
        "exit": verdict.exit,
        "wrapper": wrapper_value,
        "child": child_value,
    });
    let mut bytes = serde_json::to_vec(&document).map_err(|error| json_error(&error))?;
    bytes.push(b'\n');
    writer
        .stdout(&bytes)
        .map_err(|error| super::writer::output_error(&error))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Wrapping is a constant, and a path longer than the room left is not
    /// broken across lines: a broken path cannot be pasted back into a shell.
    #[test]
    fn prose_wraps_at_a_constant_and_never_splits_a_word() {
        let long = "a".repeat(90);
        let wrapped = wrap(&format!("start {long} end"), "  ", "    ");
        let lines: Vec<&str> = wrapped.lines().collect();
        assert_eq!(lines[1], format!("    {long}"));
        assert!(wrapped.lines().all(|line| line.starts_with("  ")));
        assert_eq!(lines[2], "    end");
    }

    /// Rule 1: colour is decoration, so stripping it leaves the same text.
    #[test]
    fn colour_changes_no_character_of_the_status_word() {
        let coloured = Palette(true).status(CheckStatus::Warn);
        assert!(coloured.contains("[warn]"));
        assert!(coloured.starts_with('\u{1b}'));
        assert_eq!(Palette(false).status(CheckStatus::Warn), "[warn]");
    }
}
