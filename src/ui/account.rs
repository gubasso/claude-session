//! Deterministic account human and JSON reports.

#![allow(
    clippy::format_push_string,
    reason = "the renderer assembles one deterministic document before its sole write"
)]

use crate::{
    domain::account::{AccountFinding, AuthModeMetadata, SelectionSource},
    error::AppError,
    ui::writer::{OutputWriter, output_error},
};

pub(crate) fn login(
    writer: &OutputWriter,
    json_mode: bool,
    account: &str,
    path: &std::path::Path,
    metadata: &AuthModeMetadata,
) -> Result<(), AppError> {
    let bytes = if json_mode {
        let mut bytes = serde_json::to_vec(&serde_json::json!({
            "account": account,
            "mode": metadata.mode.spelling(),
            "path": path.display().to_string(),
            "recorded_at": metadata.recorded_at.as_str(),
        }))
        .map_err(|error| {
            AppError::new(
                crate::error::ErrorKind::Internal,
                crate::error::Diagnostic::new(
                    "account JSON failed",
                    "account login document",
                    error.to_string(),
                    "report this wrapper bug",
                ),
            )
        })?;
        bytes.push(b'\n');
        bytes
    } else {
        format!(
            "account: {account}\nmode: {}\npath: {}\nrecorded_at: {}\n",
            metadata.mode.spelling(),
            path.display(),
            metadata.recorded_at.as_str()
        )
        .into_bytes()
    };
    writer.stdout(&bytes).map_err(|error| output_error(&error))
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
