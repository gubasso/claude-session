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
    ui::{
        prose::{BODY_INDENT, Palette, plural, wrap},
        writer::{Color, OutputWriter},
    },
};

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
        let palette = Palette::new(color.stdout());
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
    let palette = Palette::new(color.stdout());
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

    let colour = |text: String| palette.recolour(text, result.status);

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

    /// The row layout is this renderer's, even though the wrap is shared: a
    /// status word pads to the body column whatever the word is.
    #[test]
    fn every_status_word_pads_to_the_one_body_column() {
        for status in [
            CheckStatus::Pass,
            CheckStatus::Warn,
            CheckStatus::Fail,
            CheckStatus::Skipped,
        ] {
            let token = Palette::new(false).status(status);
            assert!(token.chars().count() + 2 <= BODY_INDENT, "{token}");
        }
    }
}
