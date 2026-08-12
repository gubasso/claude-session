//! Deterministic account human and JSON reports.

#![allow(
    clippy::format_push_string,
    reason = "the renderer assembles one deterministic document before its sole write"
)]

use crate::{
    domain::{
        account::{AccountFinding, AccountStatus, AuthModeMetadata, Removal, SelectionSource},
        secret::Fingerprint,
    },
    error::AppError,
    ui::writer::{OutputWriter, output_error},
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

pub(crate) fn login(
    writer: &OutputWriter,
    json_mode: bool,
    account: &str,
    path: &std::path::Path,
    metadata: &AuthModeMetadata,
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
        });
        optional(&mut value, "fingerprint", fingerprint.map(Into::into));
        optional(&mut value, "estimated_expiry", expiry.map(Into::into));
        document(&value, "account login document")?
    } else {
        let mut text = format!(
            "account: {account}\nmode: {}\npath: {}\nrecorded_at: {}\n",
            metadata.mode.spelling(),
            path.display(),
            metadata.recorded_at.as_str()
        );
        if let Some(fingerprint) = fingerprint {
            text.push_str(&format!("fingerprint: {fingerprint}\n"));
        }
        if let Some(expiry) = expiry {
            text.push_str(&format!("estimated_expiry: {expiry}\n"));
        }
        text.into_bytes()
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
        status_text(status, &warnings)
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

/// The status report's human form, carrying the same fields as labelled lines.
fn status_text(status: &AccountStatus, warnings: &[String]) -> Vec<u8> {
    {
        let mut text = format!(
            "account: {}\nselected: {}\nmode: {}\nusable: {}\n",
            status.account.as_str(),
            status.selected,
            status.mode.spelling(),
            status.usable
        );
        if let Some(source) = status.selection_source {
            text.push_str(&format!("selection_source: {}\n", source.spelling()));
        }
        if let Some(at) = &status.recorded_at {
            text.push_str(&format!("recorded_at: {}\n", at.as_str()));
        }
        if let Some(age) = status.age_seconds {
            text.push_str(&format!("age_seconds: {age}\n"));
        }
        if let Some(expiry) = &status.estimated_expiry {
            text.push_str(&format!("estimated_expiry: {expiry}\n"));
        }
        if let Some(fingerprint) = &status.fingerprint {
            text.push_str(&format!("fingerprint: {}\n", fingerprint.as_str()));
        }
        if let Some(consistent) = status.metadata_consistent {
            text.push_str(&format!("metadata_consistent: {consistent}\n"));
        }
        if let Some(present) = status.child_login_present {
            text.push_str(&format!("child_login_present: {present}\n"));
        }
        if let Some(probe) = status.child_probe {
            text.push_str(&format!("child_probe: {}\n", probe.status.spelling()));
            if let Some(code) = probe.exit_code {
                text.push_str(&format!("child_probe_exit_code: {code}\n"));
            }
        }
        for warning in warnings {
            text.push_str(&format!("warning: {warning}\n"));
        }
        text.into_bytes()
    }
}

/// Renders what one removal did, or that nothing was removed.
pub(crate) fn removal(
    writer: &OutputWriter,
    json_mode: bool,
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
        let mut text = format!(
            "account: {}\npath: {}\nremoved: {}\n",
            removal.account.as_str(),
            removal.path.display(),
            removal.removed
        );
        if let Some(mode) = removal.mode {
            text.push_str(&format!("mode: {}\n", mode.spelling()));
        }
        if let Some(cleared) = removal.marker_cleared {
            text.push_str(&format!("marker_cleared: {cleared}\n"));
        }
        text.into_bytes()
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
        let mut text = format!("selection_source: {}\n", source.spelling());
        for account in accounts {
            text.push_str(&format!(
                "account: {}\nmode: {}\nusable: {}\n",
                account.name.as_str(),
                account.mode.spelling(),
                account.usable
            ));
            if account.selected {
                text.push_str("selected: true\n");
            }
        }
        text.into_bytes()
    };
    writer.stdout(&bytes).map_err(|error| output_error(&error))
}
