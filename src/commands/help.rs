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
/// One routed verb composes: `doctor` is the single wrapper verb whose name the
/// child also owns, so its help carries the child's own after the delimiter,
/// exactly as the root spelling does (`cli-surface.md#help`). Every other node
/// appends nothing — `account` was renamed precisely so the child owns no
/// surface of that name, and the rest are wrapper-only. Requested help is a
/// result, which is why this writes to standard output and completes with `0`
/// rather than raising `Usage`, and why an unavailable child costs the section
/// rather than the status.
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
    if let Some(arguments) = child_help(verb, subcommand) {
        context
            .writer()
            .delimiter(&arguments.join(" "))
            .map_err(|error| output_error(&error))?;
        let resolved =
            crate::services::child::invocation(context, arguments.iter().map(Into::into).collect());
        compose::child_section(context, resolved)?;
    }
    Ok(DispatchOutcome::Complete(0))
}

/// The child command whose help this node's help composes, if any.
///
/// `doctor` is the one verb name the child also owns
/// (`cli-surface.md#when-the-child-owns-the-same-name`), so it is the one node
/// with a second half. The same words spell the delimiter and the child's
/// argument vector, which is what keeps the delimiter naming the exact command
/// that produced what follows (`logging-and-output.md#composed-output`).
const fn child_help(verb: &str, subcommand: Option<&str>) -> Option<&'static [&'static str]> {
    match (verb.as_bytes(), subcommand) {
        (b"doctor", None) => Some(&["doctor", "--help"]),
        _ => None,
    }
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
