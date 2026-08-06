//! Composed wrapper and native help.

use clap::CommandFactory;

use crate::{
    cli::Cli,
    commands::{compose, dispatch::DispatchOutcome},
    context::AppContext,
    error::AppError,
    ui::writer::output_error,
};

/// Emits parser-derived wrapper help then unchanged native help bytes.
pub(crate) fn run(context: &AppContext) -> Result<DispatchOutcome, AppError> {
    let mut bytes = Vec::new();
    Cli::command()
        .write_long_help(&mut bytes)
        .map_err(|error| {
            AppError::new(
                crate::error::ErrorKind::Io,
                crate::error::Diagnostic::new(
                    "help could not be rendered",
                    "standard output",
                    error.to_string(),
                    "retry the invocation",
                ),
            )
        })?;
    context
        .writer()
        .stdout(&bytes)
        .map_err(|error| output_error(&error))?;
    context
        .writer()
        .delimiter("--help")
        .map_err(|error| output_error(&error))?;
    let resolved = crate::services::child::invocation(context, vec!["--help".into()]);
    compose::child_section(context, resolved)?;
    Ok(DispatchOutcome::Complete(0))
}
