//! Deterministic profile listing reports.

use crate::{
    domain::{config::Source, encoding::with_lossy_sibling},
    error::AppError,
    services::profile::ProfileFinding,
    ui::{
        prose::{Palette, paragraph},
        writer::{Color, OutputWriter, output_error},
    },
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
    color: Color,
    source: Source,
    profiles: &[ProfileFinding],
) -> Result<(), AppError> {
    let bytes = if json_mode {
        document(profiles)?
    } else {
        text(Palette::new(color.stdout()), source, profiles)
    };
    writer.stdout(&bytes).map_err(|error| output_error(&error))
}

/// Names the layer that selected a profile, as the end of a sentence.
const fn chose(source: Source) -> &'static str {
    match source {
        Source::Cli => "because you named it with --profile",
        Source::Environment => {
            "because CLAUDE_SESSION_DEFAULT_PROFILE names it in your environment"
        }
        Source::Project => "because the project configuration file in this tree names it",
        Source::Account => "because it is the profile the selected account is bound to",
        Source::User => "because default_profile names it in your configuration file",
        Source::Default => "",
    }
}

fn text(palette: Palette, source: Source, profiles: &[ProfileFinding]) -> Vec<u8> {
    let mut out = format!("{}\n\n", palette.heading("Profiles"));
    if profiles.is_empty() {
        out.push_str(&paragraph(concat!(
            "No profiles are written yet. Every launch needs one, so write the",
            " first as a YAML document under the profiles directory, then bind",
            " an account to it with: claude-session account bind <account>",
            " --profile <name>"
        )));
        return out.into_bytes();
    }
    for profile in profiles {
        out.push_str(&paragraph(&format!(
            "{} — {}",
            profile.name.as_str(),
            profile.path.display()
        )));
    }
    out.push('\n');
    out.push_str(&paragraph(
        &profiles
            .iter()
            .find(|profile| profile.selected)
            .map_or_else(
                || {
                    concat!(
                        "None of them is selected, so a launch refuses until one",
                        " is. Bind one to the account with: claude-session",
                        " account bind <account> --profile <name>"
                    )
                    .to_owned()
                },
                |profile| {
                    format!(
                        "This run would use {}, {}.",
                        profile.name.as_str(),
                        chose(source)
                    )
                },
            ),
    ));
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
