//! One man page generated from the wrapper's own parser tree.

use clap::CommandFactory;

use crate::{
    cli::Cli, commands::dispatch::DispatchOutcome, context::AppContext, error::AppError,
    ui::writer::output_error,
};

/// Emits the wrapper's roff page as the verb's whole result.
///
/// One page for one binary, on standard output alone
/// ([ADR-0086](../../docs/decisions/ADR-0086-emit-one-man-page-to-standard-output.md)).
/// The authored prose that reaches the parser's long help reaches this page
/// through the same `include_str!`, so it is written once and read twice
/// ([ADR-0016](../../docs/decisions/ADR-0016-ship-man-pages.md)).
pub(crate) fn run(context: &AppContext) -> Result<DispatchOutcome, AppError> {
    let mut command = Cli::command();
    command.build();
    let mut bytes = Vec::new();
    clap_mangen::Man::new(command)
        .render(&mut bytes)
        .map_err(|error| {
            AppError::new(
                crate::error::ErrorKind::Io,
                crate::error::Diagnostic::new(
                    "the man page could not be rendered",
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
    Ok(DispatchOutcome::Complete(0))
}
