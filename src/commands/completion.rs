//! Shell completions generated from the wrapper's own parser tree.

use clap::CommandFactory;
use clap_complete::Shell;

use crate::{
    cli::Cli, commands::dispatch::DispatchOutcome, context::AppContext, error::AppError,
    ui::writer::output_error,
};

/// Emits one shell's completion script as the verb's whole result.
///
/// The script is written raw: no header, no summary, no diagnostic
/// (`cli-surface.md#help`), and no decoration, because the coloured set is
/// closed and this surface is not in it (`presentation.md`). The generator reads
/// the same `Command` tree that produces help and man pages, so the flag list
/// has one source and no surface can drift from another.
pub(crate) fn run(context: &AppContext, shell: Shell) -> Result<DispatchOutcome, AppError> {
    let mut command = Cli::command();
    command.build();
    let mut bytes = Vec::new();
    clap_complete::generate(shell, &mut command, "claude-session", &mut bytes);
    context
        .writer()
        .stdout(&bytes)
        .map_err(|error| output_error(&error))?;
    Ok(DispatchOutcome::Complete(0))
}
