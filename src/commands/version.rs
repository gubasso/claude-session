//! Composed human and JSON version inspection.

use crate::{
    adapters::process::ProcessRunner,
    commands::dispatch::DispatchOutcome,
    context::AppContext,
    error::{AppError, Diagnostic},
};
use serde::Serialize;

#[derive(Serialize)]
struct VersionDocument<'a> {
    version: &'a str,
    child: ChildVersion,
}
#[derive(Serialize)]
struct ChildVersion {
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<String>,
}

/// Emits wrapper and child version information without making inspection fail.
// The nested result distinguishes resolution from spawn failure.
#[allow(clippy::option_if_let_else)]
pub(crate) fn run(context: &AppContext, json: bool) -> Result<DispatchOutcome, AppError> {
    let resolved = crate::services::child::invocation(context, vec!["--version".into()]);
    if json {
        let child = match resolved {
            Ok(invocation) => match context.adapters().process().run_captured(&invocation) {
                Ok(captured) => {
                    let successful = matches!(
                        captured.outcome,
                        crate::domain::child::ChildOutcome::Exited(0)
                    );
                    if !captured.stderr.is_empty() {
                        tracing::debug!(
                            op = "inspect_version",
                            status = "ok",
                            "child wrote version diagnostics"
                        );
                    }
                    let version = std::str::from_utf8(&captured.stdout)
                        .ok()
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(str::to_owned);
                    ChildVersion {
                        status: if successful && version.is_some() {
                            "ok"
                        } else {
                            "unparsable"
                        },
                        path: Some(invocation.program().display().to_string()),
                        version,
                    }
                }
                Err(error) => ChildVersion {
                    status: resolution_status(&error),
                    path: Some(invocation.program().display().to_string()),
                    version: None,
                },
            },
            Err(error) => ChildVersion {
                status: resolution_status(&error),
                path: None,
                version: None,
            },
        };
        let document = VersionDocument {
            version: env!("CARGO_PKG_VERSION"),
            child,
        };
        let mut bytes = serde_json::to_vec(&document).map_err(|error| {
            AppError::Internal(Diagnostic::new(
                "version JSON failed",
                "version document",
                error.to_string(),
                "report this wrapper bug",
            ))
        })?;
        bytes.push(b'\n');
        context
            .writer()
            .stdout(&bytes)
            .map_err(|error| output_error(&error))?;
    } else {
        context
            .writer()
            .stdout(format!("claude-session {}\n", env!("CARGO_PKG_VERSION")).as_bytes())
            .map_err(|error| output_error(&error))?;
        if let Ok(invocation) = &resolved {
            context
                .writer()
                .stdout(format!("claude [{}]\n", invocation.program().display()).as_bytes())
                .map_err(|error| output_error(&error))?;
        }
        context
            .writer()
            .delimiter("--version")
            .map_err(|error| output_error(&error))?;
        match resolved {
            Ok(invocation) => {
                let _ = context.adapters().process().run_inherited(&invocation);
            }
            Err(error) => context
                .writer()
                .stdout(format!("claude unavailable: {}\n", error.kind().spelling()).as_bytes())
                .map_err(|output| output_error(&output))?,
        }
    }
    Ok(DispatchOutcome::Complete(0))
}

fn output_error(error: &std::io::Error) -> AppError {
    AppError::Io(Diagnostic::new(
        "terminal output failed",
        "standard output",
        error.to_string(),
        "check the output stream",
    ))
}

const fn resolution_status(error: &AppError) -> &'static str {
    match error.kind() {
        crate::error::ErrorKind::ChildNotFound => "not-found",
        crate::error::ErrorKind::ChildNotExecutable | crate::error::ErrorKind::ChildRecursion => {
            "not-executable"
        }
        _ => "unparsable",
    }
}
