//! Composed wrapper and native help.

use clap::CommandFactory;

use crate::{
    adapters::process::ProcessRunner, cli::Cli, commands::dispatch::DispatchOutcome,
    context::AppContext, error::AppError,
};

/// Emits parser-derived wrapper help then unchanged native help bytes.
pub(crate) fn run(context: &AppContext) -> Result<DispatchOutcome, AppError> {
    let mut bytes = Vec::new();
    Cli::command()
        .write_long_help(&mut bytes)
        .map_err(|error| {
            AppError::Io(crate::error::Diagnostic::new(
                "help could not be rendered",
                "standard output",
                error.to_string(),
                "retry the invocation",
            ))
        })?;
    context
        .writer()
        .stdout(&bytes)
        .map_err(|error| output_error(&error))?;
    context
        .writer()
        .delimiter("--help")
        .map_err(|error| output_error(&error))?;
    match crate::services::child::invocation(context, vec!["--help".into()]) {
        Ok(invocation) => {
            let _ = context.adapters().process().run_inherited(&invocation);
        }
        Err(error) => {
            context
                .writer()
                .stdout(format!("claude unavailable: {}\n", error.kind().spelling()).as_bytes())
                .map_err(|output| output_error(&output))?;
        }
    }
    Ok(DispatchOutcome::Complete(0))
}

fn output_error(error: &std::io::Error) -> AppError {
    AppError::Io(crate::error::Diagnostic::new(
        "terminal output failed",
        "standard output",
        error.to_string(),
        "check the output stream",
    ))
}
