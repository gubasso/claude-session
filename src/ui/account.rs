//! Deterministic account reports, in both forms.
//!
//! The two forms have two audiences and one source. `human` writes the sentences
//! a person reads; everything below is the machine document, whose field set is
//! what makes those sentences affordable to change.

#![allow(
    clippy::format_push_string,
    reason = "the renderer assembles one deterministic document before its sole write"
)]

pub(crate) mod human;

use crate::{
    domain::{
        account::{
            AccountFinding, AccountStatus, AuthModeMetadata, ProfileBinding, Removal,
            SelectionSource,
        },
        config::Source,
        identifier::Identifier,
        secret::Fingerprint,
    },
    error::AppError,
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
                "account JSON failed",
                surface,
                error.to_string(),
                "report this wrapper bug",
            ),
        )
    })?;
    bytes.push(b'\n');
    Ok(bytes)
}

/// Inserts an optional field, or leaves it absent.
///
/// Absent rather than null is the shared document rule, and doing it here means
/// no renderer decides it one field at a time.
fn optional(target: &mut serde_json::Value, key: &str, value: Option<serde_json::Value>) {
    if let Some(value) = value {
        target[key] = value;
    }
}

/// The profile a login bound, and how the report should say where it came from.
pub(crate) struct Binding {
    pub(crate) profile: Identifier,
    pub(crate) source: Source,
    pub(crate) present: bool,
}

/// Renders one account's new profile binding.
pub(crate) fn binding(
    writer: &OutputWriter,
    json_mode: bool,
    color: Color,
    account: &Identifier,
    binding: &ProfileBinding,
) -> Result<(), AppError> {
    let bytes = if json_mode {
        document(
            &serde_json::json!({
                "account": account.as_str(),
                "profile": binding.profile.as_str(),
                "recorded_at": binding.recorded_at.as_str(),
            }),
            "account bind document",
        )?
    } else {
        human::binding(Palette::new(color.stdout()), account, binding).into_bytes()
    };
    writer.stdout(&bytes).map_err(|error| output_error(&error))
}

pub(crate) fn login(
    writer: &OutputWriter,
    json_mode: bool,
    account: &str,
    path: &std::path::Path,
    metadata: &AuthModeMetadata,
    binding: &Binding,
    color: Color,
) -> Result<(), AppError> {
    let fingerprint = metadata.token_fingerprint().map(Fingerprint::as_str);
    let expiry = metadata
        .token_fingerprint()
        .and_then(|_| crate::services::account::token::estimated_expiry(&metadata.recorded_at));
    let bytes = if json_mode {
        let mut value = serde_json::json!({
            "account": account,
            "mode": metadata.mode.spelling(),
            "path": path.display().to_string(),
            "recorded_at": metadata.recorded_at.as_str(),
            "profile": binding.profile.as_str(),
            "profile_source": binding.source.spelling(),
            "profile_present": binding.present,
        });
        optional(&mut value, "fingerprint", fingerprint.map(Into::into));
        optional(&mut value, "estimated_expiry", expiry.map(Into::into));
        document(&value, "account login document")?
    } else {
        human::login(
            Palette::new(color.stdout()),
            account,
            path,
            metadata,
            binding,
        )
        .into_bytes()
    };
    writer.stdout(&bytes).map_err(|error| output_error(&error))
}

/// Renders one account's status.
///
/// This is the only report carrying `warnings` as data, because shadowing is
/// its subject. Everywhere else a warning is prose on standard error.
pub(crate) fn status(
    writer: &OutputWriter,
    json_mode: bool,
    color: Color,
    status: &AccountStatus,
) -> Result<(), AppError> {
    let warnings: Vec<String> = status
        .warnings
        .iter()
        .map(|warning| warning.message())
        .collect();
    let bytes = if json_mode {
        status_document(status, &warnings)?
    } else {
        human::status(Palette::new(color.stdout()), status).into_bytes()
    };
    writer.stdout(&bytes).map_err(|error| output_error(&error))
}

/// The status report's JSON form.
fn status_document(status: &AccountStatus, warnings: &[String]) -> Result<Vec<u8>, AppError> {
    {
        let mut value = serde_json::json!({
            "account": status.account.as_str(),
            "selected": status.selected,
            "mode": status.mode.spelling(),
            "usable": status.usable,
            "warnings": warnings,
        });
        optional(
            &mut value,
            "selection_source",
            status
                .selection_source
                .map(|source| source.spelling().into()),
        );
        optional(
            &mut value,
            "profile",
            status.profile.as_ref().map(|name| name.as_str().into()),
        );
        optional(
            &mut value,
            "profile_present",
            status.profile_present.map(Into::into),
        );
        optional(
            &mut value,
            "profile_source",
            status.profile_source.map(|source| source.spelling().into()),
        );
        optional(
            &mut value,
            "recorded_at",
            status.recorded_at.as_ref().map(|at| at.as_str().into()),
        );
        optional(
            &mut value,
            "age_seconds",
            status.age_seconds.map(Into::into),
        );
        optional(
            &mut value,
            "estimated_expiry",
            status.estimated_expiry.clone().map(Into::into),
        );
        optional(
            &mut value,
            "fingerprint",
            status.fingerprint.as_ref().map(|f| f.as_str().into()),
        );
        optional(
            &mut value,
            "metadata_consistent",
            status.metadata_consistent.map(Into::into),
        );
        optional(
            &mut value,
            "child_login_present",
            status.child_login_present.map(Into::into),
        );
        optional(
            &mut value,
            "child_probe",
            status.child_probe.map(|probe| {
                let mut value = serde_json::json!({"status": probe.status.spelling()});
                optional(&mut value, "exit_code", probe.exit_code.map(Into::into));
                value
            }),
        );
        document(&value, "account status document")
    }
}

/// Renders what one removal did, or that nothing was removed.
pub(crate) fn removal(
    writer: &OutputWriter,
    json_mode: bool,
    color: Color,
    removal: &Removal,
) -> Result<(), AppError> {
    let bytes = if json_mode {
        let mut value = serde_json::json!({
            "account": removal.account.as_str(),
            "path": removal.path.display().to_string(),
            "removed": removal.removed,
        });
        optional(
            &mut value,
            "mode",
            removal.mode.map(|mode| mode.spelling().into()),
        );
        optional(
            &mut value,
            "marker_cleared",
            removal.marker_cleared.map(Into::into),
        );
        document(&value, "account remove document")?
    } else {
        human::removal(Palette::new(color.stdout()), removal).into_bytes()
    };
    writer.stdout(&bytes).map_err(|error| output_error(&error))
}

/// Warns, before prompting, that the account about to go is the selected one.
pub(crate) fn selected_removal_warning(account: &str) {
    tracing::warn!(
        "account {account} is the currently selected account; removing it leaves nothing selected"
    );
}

/// States the two things a removal did not do.
///
/// Written straight to standard error rather than through the log mirror, which
/// JSON mode turns off. That would be the wrong trade here: no report field
/// carries either fact — one would be `false` in both modes forever and
/// discriminate nothing — so the sentence is the only place it exists, and a
/// formatting flag must not be what deletes a safety statement. The prompt is
/// already off standard output for the same reason, so this disturbs no
/// `--json` pipeline. No page or URL is named: the wrapper cannot verify one.
pub(crate) fn removal_consequences(writer: &OutputWriter, removal: &Removal) {
    if !removal.removed {
        return;
    }
    let credential = match removal.mode {
        Some(crate::domain::account::ReportMode::Token) => {
            "the stored token keeps working wherever else it is used"
        }
        Some(crate::domain::account::ReportMode::Login) => {
            "the child-owned saved login is not ended by deleting it"
        }
        _ => "any credential this account held is not ended by deleting it",
    };
    let _ = writer.stderr(
        format!(
            concat!(
                "claude-session-rs: warning: a session already running on this account",
                " keeps working until it exits; its next start will fail\n",
                "claude-session-rs: warning: removing local state is not upstream",
                " revocation: {}; revoke it at the provider if that is what you meant\n"
            ),
            credential
        )
        .as_bytes(),
    );
}

pub(crate) fn list(
    writer: &OutputWriter,
    json_mode: bool,
    color: Color,
    source: SelectionSource,
    accounts: &[AccountFinding],
) -> Result<(), AppError> {
    let bytes = if json_mode {
        let values: Vec<_> = accounts
            .iter()
            .map(|account| {
                let mut value = serde_json::json!({
                    "name": account.name.as_str(),
                    "mode": account.mode.spelling(),
                    "usable": account.usable,
                });
                optional(
                    &mut value,
                    "profile",
                    account.profile.as_ref().map(|name| name.as_str().into()),
                );
                if account.selected {
                    value["selected"] = serde_json::Value::Bool(true);
                }
                value
            })
            .collect();
        let mut bytes = serde_json::to_vec(
            &serde_json::json!({"selection_source": source.spelling(), "accounts": values}),
        )
        .map_err(|error| {
            AppError::new(
                crate::error::ErrorKind::Internal,
                crate::error::Diagnostic::new(
                    "account JSON failed",
                    "account list document",
                    error.to_string(),
                    "report this wrapper bug",
                ),
            )
        })?;
        bytes.push(b'\n');
        bytes
    } else {
        human::list(Palette::new(color.stdout()), source, accounts).into_bytes()
    };
    writer.stdout(&bytes).map_err(|error| output_error(&error))
}
