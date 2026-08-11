//! Discovery of the profiles a user could select with `--profile`.
//!
//! Discovery only. The documents are deliberately not opened here: listing
//! reports which names exist, and parsing one to report it would claim
//! composition behaviour the listing does not perform. `config` owns the
//! per-profile report.

use std::{fs, path::PathBuf};

use crate::{
    context::AppContext,
    domain::identifier::Identifier,
    error::{AppError, Diagnostic, ErrorKind},
};

/// One profile the user could select with `--profile`.
#[derive(Clone, Debug)]
pub(crate) struct ProfileFinding {
    /// The profile's name, which is its file stem.
    pub(crate) name: Identifier,
    /// The profile document's path.
    pub(crate) path: PathBuf,
    /// Whether this profile is the selected one.
    pub(crate) selected: bool,
}

/// Enumerates the profiles available to `--profile`, in name order.
///
/// An absent directory is an empty list rather than a failure: `profile` is an
/// inspection verb, and a user who has written no profile yet has asked a
/// question with the answer "none" (`exit-codes.md`).
pub(crate) fn discover(context: &AppContext) -> Result<Vec<ProfileFinding>, AppError> {
    let root = context.paths().profiles();
    let entries = match fs::read_dir(&root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(read_error(&root, &error)),
    };
    let selected = context.session().profile();
    let mut findings = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| read_error(&root, &error))?;
        // A stray file in a user-owned configuration directory is not a wrapper
        // failure, so every rejection below is a skip rather than a diagnostic.
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if !kind.is_file() {
            continue;
        }
        let name = entry.file_name();
        let Some(text) = name.to_str() else { continue };
        let Some(stem) = text.strip_suffix(".yaml") else {
            continue;
        };
        let Ok(name) = stem.parse::<Identifier>() else {
            continue;
        };
        findings.push(ProfileFinding {
            selected: selected == Some(&name),
            path: entry.path(),
            name,
        });
    }
    findings.sort_by(|left, right| left.name.as_str().cmp(right.name.as_str()));
    Ok(findings)
}

fn read_error(root: &std::path::Path, error: &std::io::Error) -> AppError {
    AppError::new(
        ErrorKind::Io,
        Diagnostic::new(
            "the profile directory could not be read",
            root.display().to_string(),
            error.to_string(),
            "check the path and the filesystem",
        ),
    )
}
