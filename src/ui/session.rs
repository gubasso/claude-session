//! Deterministic session reports, in both forms.
//!
//! The two forms have two audiences and one source. `human` writes the
//! sentences a person reads; everything below is the machine document, whose
//! field set is what makes those sentences affordable to change.

pub(crate) mod human;

use crate::{
    error::AppError,
    services::session::gc::SessionFinding,
    ui::{
        prose::Palette,
        writer::{Color, OutputWriter, output_error},
    },
};

/// Encodes one report document, naming the surface that failed.
fn document(value: &serde_json::Value, surface: &'static str) -> Result<Vec<u8>, AppError> {
    let mut bytes = serde_json::to_vec(value).map_err(|error| {
        AppError::new(
            crate::error::ErrorKind::Internal,
            crate::error::Diagnostic::new(
                "session JSON failed",
                surface,
                error.to_string(),
                "report this wrapper bug",
            ),
        )
    })?;
    bytes.push(b'\n');
    Ok(bytes)
}

/// Projects one finding into its document row.
fn finding_row(finding: &SessionFinding) -> serde_json::Value {
    let mut value = serde_json::json!({
        "account": finding.account.as_str(),
        "namespace": finding.namespace.as_str(),
        "terminal": finding.terminal.as_str(),
        "verdict": finding.verdict.as_str(),
        // Always present: every finding stands on a ground, and the verdict is
        // that ground's projection.
        "ground": finding.ground.spelling(),
        "current": finding.current,
        "path": finding.path.display().to_string(),
    });
    // Absent when no witness could be read, so the key discriminates a judged
    // record from a directory that predates one ([ADR-0051]).
    //
    // [ADR-0051]: ../../docs/decisions/ADR-0051-let-every-surface-element-discriminate.md
    if let Some(source) = finding.source {
        value["rung"] = source.spelling().into();
    }
    // Absent for the same reason: a record that names no terminal has no name
    // to report, and a caller can tell that from the key rather than from a
    // sentinel value.
    if let Some(named) = &finding.named {
        value["names"] = named.as_str().into();
    }
    value
}

/// Renders every session directory and its verdict.
pub(crate) fn list(
    writer: &OutputWriter,
    json_mode: bool,
    color: Color,
    findings: &[SessionFinding],
) -> Result<(), AppError> {
    let bytes = if json_mode {
        let rows: Vec<_> = findings.iter().map(finding_row).collect();
        document(
            &serde_json::json!({"sessions": rows}),
            "session list document",
        )?
    } else {
        human::list(Palette::new(color.stdout()), findings).into_bytes()
    };
    writer.stdout(&bytes).map_err(|error| output_error(&error))
}

/// Renders the one question `session clean` asks before it removes anything.
///
/// Never a document: the prompt reaches the controlling terminal rather than
/// either standard stream, so a `--json` consumer reading standard output
/// never sees it ([presentation]).
///
/// [presentation]: ../../docs/reference/presentation.md
pub(crate) fn prompt(collectable: &[SessionFinding]) -> String {
    human::prompt(collectable)
}

/// Renders what one `clean` removed, or that nothing was.
///
/// `declined` distinguishes a refusal at the prompt from a tree with nothing
/// dead in it; both remove nothing, and both are outcomes rather than errors.
pub(crate) fn collection(
    writer: &OutputWriter,
    json_mode: bool,
    color: Color,
    removed: &[SessionFinding],
    pruned_namespaces: usize,
    declined: bool,
) -> Result<(), AppError> {
    let bytes = if json_mode {
        let rows: Vec<_> = removed.iter().map(finding_row).collect();
        let mut value = serde_json::json!({
            "removed": rows,
            "pruned_namespaces": pruned_namespaces,
        });
        if declined {
            value["declined"] = serde_json::Value::Bool(true);
        }
        document(&value, "session clean document")?
    } else {
        human::collection(
            Palette::new(color.stdout()),
            removed,
            pruned_namespaces,
            declined,
        )
        .into_bytes()
    };
    writer.stdout(&bytes).map_err(|error| output_error(&error))
}
