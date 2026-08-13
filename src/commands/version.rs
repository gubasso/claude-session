//! Composed human and JSON version inspection.

use crate::{
    adapters::process::ProcessRunner,
    commands::{
        compose,
        dispatch::{DispatchOutcome, OutputMode},
    },
    context::AppContext,
    error::{AppError, Diagnostic},
    ui::writer::output_error,
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
pub(crate) fn run(context: &AppContext) -> Result<DispatchOutcome, AppError> {
    let resolved = crate::services::child::invocation(context, vec!["--version".into()]);
    if context.output_mode() == OutputMode::Json {
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
                    let version = crate::domain::child::ChildVersion::parse(&captured.stdout)
                        .map(|value| value.to_string());
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
            AppError::new(
                crate::error::ErrorKind::Internal,
                Diagnostic::new(
                    "version JSON failed",
                    "version document",
                    error.to_string(),
                    "report this wrapper bug",
                ),
            )
        })?;
        bytes.push(b'\n');
        context
            .writer()
            .stdout(&bytes)
            .map_err(|error| output_error(&error))?;
    } else {
        let mut text = crate::ui::prose::paragraph(&format!(
            "This is claude-session-rs {}.",
            env!("CARGO_PKG_VERSION")
        ));
        text.push_str(&crate::ui::prose::paragraph(&match &resolved {
            Ok(invocation) => format!(
                "It wraps the claude at {}, whose own version follows.",
                invocation.program().display()
            ),
            Err(_) => "No claude could be resolved, so there is nothing below to wrap.".to_owned(),
        }));
        context
            .writer()
            .stdout(text.as_bytes())
            .map_err(|error| output_error(&error))?;
        context
            .writer()
            .delimiter("--version")
            .map_err(|error| output_error(&error))?;
        compose::child_section(context, resolved)?;
    }
    Ok(DispatchOutcome::Complete(0))
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
