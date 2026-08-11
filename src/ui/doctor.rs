//! Deterministic human and JSON projections of the doctor catalog.

#![allow(
    clippy::format_collect,
    clippy::format_push_string,
    reason = "the renderer assembles one deterministic document before its sole write"
)]

use serde_json::{Value, json};

use crate::{
    domain::checks::{CATALOG, CheckResult, CheckStatus, ChildReport, Summary, Verdict},
    error::{AppError, Diagnostic, ErrorKind},
    ui::writer::OutputWriter,
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
pub(crate) fn list(writer: &OutputWriter, json_mode: bool) -> Result<(), AppError> {
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
        CATALOG
            .iter()
            .map(|check| {
                format!(
                    "{} {} {}\n",
                    check.id(),
                    check.scope().as_str(),
                    check.severity().as_str()
                )
            })
            .collect::<String>()
            .into_bytes()
    };
    writer
        .stdout(&bytes)
        .map_err(|error| super::writer::output_error(&error))
}

/// Writes one complete report and its composed child section.
pub(crate) fn report(
    writer: &OutputWriter,
    json_mode: bool,
    results: &[CheckResult],
    summary: Summary,
    child: &ChildReport,
    verdict: Verdict,
) -> Result<(), AppError> {
    if json_mode {
        return json_report(writer, results, summary, child, verdict);
    }
    let mut text = String::new();
    for scope in ["host", "session"] {
        text.push_str(if scope == "host" {
            "Host\n"
        } else {
            "Session\n"
        });
        for result in results
            .iter()
            .filter(|result| result.check.scope().as_str() == scope)
        {
            text.push_str(&format!(
                "[{}] {} {}\n",
                result.status.as_str(),
                result.check.id(),
                result.message
            ));
            if let Some(hint) = &result.hint {
                text.push_str(&format!("  hint: {hint}\n"));
            }
            if let Some(reason) = &result.reason {
                text.push_str(&format!("  reason: {reason}\n"));
            }
        }
    }
    // The three levels a composed run has: this wrapper's own catalog, the
    // child's own report, and the verdict over both. Each states its own code,
    // so the process status is never explained by a value the report omits.
    text.push_str(&format!(
        "wrapper status={} total={} passed={} warned={} failed={} skipped={} \
hard_failures={} exit={}\n",
        summary.status().as_str(),
        summary.total,
        summary.passed,
        summary.warned,
        summary.failed,
        summary.skipped,
        summary.hard_failures,
        summary.exit
    ));
    text.push_str(&format!("child status={}", child.status.as_str()));
    if let Some(exit) = child.exit {
        text.push_str(&format!(" exit={exit}"));
    }
    if let Some(reason) = &child.reason {
        text.push_str(&format!(" reason={reason}"));
    }
    text.push('\n');
    text.push_str(&format!(
        "doctor status={} wrapper={} child={} exit={}\n",
        verdict.status.as_str(),
        summary.exit,
        child
            .exit
            .map_or_else(|| "none".to_owned(), |exit| exit.to_string()),
        verdict.exit
    ));
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
