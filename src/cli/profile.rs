//! Parse shape for the wrapper-owned profile listing.

use clap::{ArgAction, Args};

/// Profile listing output flags.
///
/// No positional and no subcommand: the bare verb is the whole grammar
/// ([ADR-0051](../../docs/decisions/ADR-0051-let-every-surface-element-discriminate.md)),
/// and one profile's own report belongs to `config --profile <name>`.
#[derive(Args, Debug)]
pub(crate) struct ProfileArgs {
    /// Emit the report as one JSON document.
    #[arg(long)]
    pub(crate) json: bool,
    // A distinct id, because clap propagates a global argument's value into the
    // parent match and a shared `help` id would also set the root flag, which
    // would answer with the composed wrapper help instead of this verb's.
    /// Print this command's help.
    #[arg(id = "profile_help", long = "help", short = 'h', action = ArgAction::SetTrue)]
    pub(crate) help_flag: bool,
}
