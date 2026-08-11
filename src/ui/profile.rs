//! Deterministic profile listing reports.

use crate::{
    domain::encoding::with_lossy_sibling,
    error::AppError,
    services::profile::ProfileFinding,
    ui::writer::{OutputWriter, output_error},
};

/// Renders the available profile names.
///
/// One line per profile, with no padding and no table: layout that depends on
/// the terminal's width carries meaning the plain text would then be missing
/// (`presentation.md`). An empty list writes zero bytes, because "none" is the
/// answer rather than a diagnostic.
pub(crate) fn list(
    writer: &OutputWriter,
    json_mode: bool,
    profiles: &[ProfileFinding],
) -> Result<(), AppError> {
    let bytes = if json_mode {
        document(profiles)?
    } else {
        text(profiles)
    };
    writer.stdout(&bytes).map_err(|error| output_error(&error))
}

fn text(profiles: &[ProfileFinding]) -> Vec<u8> {
    let mut out = String::new();
    for profile in profiles {
        out.push_str(profile.name.as_str());
        if profile.selected {
            out.push_str(" (selected)");
        }
        out.push('\n');
    }
    out.into_bytes()
}

fn document(profiles: &[ProfileFinding]) -> Result<Vec<u8>, AppError> {
    let values: Vec<serde_json::Value> = profiles
        .iter()
        .map(|profile| {
            let mut value = serde_json::json!({
                "name": profile.name.as_str(),
                "path": profile.path.display().to_string(),
            });
            with_lossy_sibling(&mut value, "path", &profile.path);
            // Omitted rather than false: an absent optional field is the shared
            // document rule (`logging-and-output.md`).
            if profile.selected {
                value["selected"] = serde_json::Value::Bool(true);
            }
            value
        })
        .collect();
    let mut bytes =
        serde_json::to_vec(&serde_json::json!({ "profiles": values })).map_err(|error| {
            AppError::new(
                crate::error::ErrorKind::Internal,
                crate::error::Diagnostic::new(
                    "profile JSON failed",
                    "profile list document",
                    error.to_string(),
                    "report this wrapper bug",
                ),
            )
        })?;
    bytes.push(b'\n');
    Ok(bytes)
}
