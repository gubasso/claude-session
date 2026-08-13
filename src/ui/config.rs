//! Deterministic configuration reports, in both forms.
//!
//! `human` writes the sentences; everything below is the machine document.

#![allow(
    clippy::format_push_string,
    reason = "the renderer assembles one deterministic document before its sole write"
)]

pub(crate) mod human;

use crate::{
    commands::config::Report,
    domain::encoding::with_lossy_sibling,
    error::AppError,
    ui::{
        prose::Palette,
        writer::{Color, OutputWriter, output_error},
    },
};

/// Renders the resolved configuration, its files, the active profile, and its
/// defects.
///
/// Both forms project from one `Report`, so neither can omit a field the other
/// carries. No colour: `config` renders none of the closed set of coloured
/// surfaces (`presentation.md`).
pub(crate) fn report(
    writer: &OutputWriter,
    json_mode: bool,
    color: Color,
    report: &Report,
) -> Result<(), AppError> {
    let bytes = if json_mode {
        document(report)?
    } else {
        human::report(Palette::new(color.stdout()), report).into_bytes()
    };
    writer.stdout(&bytes).map_err(|error| output_error(&error))
}

/// Renders the profile section, which is omitted when no name resolved.
fn profile_section(profile: &crate::commands::config::ProfileReport) -> serde_json::Value {
    let pieces: Vec<serde_json::Value> = profile
        .pieces
        .iter()
        .map(|piece| {
            let mut entry = serde_json::json!({
                "name": piece.name.as_str(),
                "path": piece.path.display().to_string(),
            });
            with_lossy_sibling(&mut entry, "path", &piece.path);
            entry
        })
        .collect();
    let strategies: Vec<serde_json::Value> = profile
        .strategies
        .iter()
        .map(|strategy| {
            let mut entry = serde_json::json!({
                "pointer": strategy.pointer,
                "strategy": strategy.strategy,
            });
            if let Some(key) = strategy.key.as_ref() {
                entry["key"] = serde_json::Value::String(key.clone());
            }
            entry
        })
        .collect();
    let mut entry = serde_json::json!({
        "settings": profile.settings.display().to_string(),
        "provenance": profile.provenance.display().to_string(),
        "digest": profile.digest,
        "exists": profile.exists,
    });
    with_lossy_sibling(&mut entry, "settings", &profile.settings);
    with_lossy_sibling(&mut entry, "provenance", &profile.provenance);
    let mut section = serde_json::json!({
        "name": profile.name.as_str(),
        "path": profile.path.display().to_string(),
        "pieces": pieces,
        "strategies": strategies,
        "entry": entry,
    });
    with_lossy_sibling(&mut section, "path", &profile.path);
    section
}

/// Renders the config-scoped catalog results.
fn defect_rows(report: &Report) -> Vec<serde_json::Value> {
    report
        .defects
        .iter()
        .map(|defect| {
            let mut entry = serde_json::json!({
                "id": defect.check.id(),
                "status": defect.status.as_str(),
            });
            if let Some(hint) = defect.hint.as_ref() {
                entry["hint"] = serde_json::Value::String(hint.clone());
            }
            if let Some(reason) = defect.reason.as_ref() {
                entry["reason"] = serde_json::Value::String(reason.clone());
            }
            entry
        })
        .collect()
}

fn document(report: &Report) -> Result<Vec<u8>, AppError> {
    let mut configuration = serde_json::Map::new();
    for key in &report.configuration {
        let mut entry = serde_json::json!({ "source": key.source.spelling() });
        if let Some(value) = key.value.as_ref() {
            entry["value"] = serde_json::Value::String(value.clone());
        }
        configuration.insert(key.name.to_owned(), entry);
    }
    let files: Vec<serde_json::Value> = report
        .files
        .iter()
        .map(|file| {
            let mut entry = serde_json::json!({
                "path": file.path.display().to_string(),
                "layer": file.source.spelling(),
                "existed": file.existed,
            });
            with_lossy_sibling(&mut entry, "path", &file.path);
            entry
        })
        .collect();
    // Arrays are present even when empty; a genuinely optional object is
    // omitted (`logging-and-output.md`). No `schema_version`: that appears on
    // `doctor` alone.
    let mut value = serde_json::json!({
        "configuration": serde_json::Value::Object(configuration),
        "files": files,
        "defects": defect_rows(report),
    });
    if let Some(profile) = report.profile.as_ref() {
        value["profile"] = profile_section(profile);
    }
    let mut bytes = serde_json::to_vec(&value).map_err(|error| {
        AppError::new(
            crate::error::ErrorKind::Internal,
            crate::error::Diagnostic::new(
                "config JSON failed",
                "config report document",
                error.to_string(),
                "report this wrapper bug",
            ),
        )
    })?;
    bytes.push(b'\n');
    Ok(bytes)
}
