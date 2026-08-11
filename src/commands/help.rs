//! Composed wrapper and native help.

use clap::CommandFactory;

use crate::{
    cli::Cli,
    commands::{compose, dispatch::DispatchOutcome},
    context::AppContext,
    error::AppError,
    ui::writer::output_error,
};

/// Emits one verb node's parser-derived help as a result.
///
/// Nothing is composed onto it: no verb routed here appears in the child's
/// inventory — `account` was renamed precisely so the child owns no surface of
/// that name, and `completion` and `man` are wrapper-only (`cli-surface.md#help`)
/// — so there is no child help to append. Requested help is a result, which is
/// why this writes to standard output and completes with `0` rather than
/// raising `Usage`.
pub(crate) fn verb(
    context: &AppContext,
    verb: &str,
    subcommand: Option<&str>,
) -> Result<DispatchOutcome, AppError> {
    let mut root = Cli::command();
    root.build();
    let node = root
        .find_subcommand_mut(verb)
        .ok_or_else(|| render_error("the request names no wrapper verb"))?;
    let node = match subcommand {
        None => node,
        Some(name) => node
            .find_subcommand_mut(name)
            .ok_or_else(|| render_error("the verb has no such subcommand"))?,
    };
    let mut bytes = node.render_long_help().to_string().into_bytes();
    bytes.push(b'\n');
    context
        .writer()
        .stdout(&bytes)
        .map_err(|error| output_error(&error))?;
    Ok(DispatchOutcome::Complete(0))
}

fn render_error(why: &str) -> AppError {
    AppError::new(
        crate::error::ErrorKind::Internal,
        crate::error::Diagnostic::new(
            "help could not be rendered",
            "standard output",
            why,
            "report this: the parser tree and the dispatcher disagree",
        ),
    )
}

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
