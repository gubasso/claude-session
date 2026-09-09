//! Parse shape for session lifecycle management.

use crate::domain::registration::Subject;
use clap::{ArgAction, Args, Subcommand};

#[derive(Args, Debug)]
pub(crate) struct SessionArgs {
    // Requested help is a result rather than a diagnostic
    // (`cli-surface.md#help`), so the parser's own help flag stays disabled
    // and the request is carried as a value the dispatcher answers on standard
    // output at exit `0`. The flag is global inside this namespace so
    // `session list --help` reaches the subcommand's own help rather than the
    // namespace's. The id is distinct from the root flag's on purpose: clap
    // propagates a global argument's value up into the parent matches as well
    // as down, so sharing an id would make `session --help` also set the
    // root's flag and answer with the composed wrapper help instead.
    /// Print this command's help.
    #[arg(
        id = "session_help",
        long = "help",
        short = 'h',
        action = ArgAction::SetTrue,
        global = true,
    )]
    pub(crate) help_flag: bool,
    // Optional only so `session --help` can parse. A bare `session` is still
    // malformed, and the dispatcher raises the `Usage` the parser used to
    // ([ADR-0052](../../docs/decisions/ADR-0052-require-an-explicit-subcommand.md)).
    #[command(subcommand)]
    pub(crate) command: Option<SessionCommand>,
}

#[derive(Subcommand, Debug)]
pub(crate) enum SessionCommand {
    /// List every session directory and its liveness.
    List(ListArgs),
    /// Remove the session directories whose terminal is provably gone.
    Clean(CleanArgs),
}

#[derive(Args, Debug)]
pub(crate) struct ListArgs {
    /// Session to report, named as its row names it; every session when omitted.
    pub(crate) name: Option<Subject>,
    /// Emit the report as one JSON document.
    #[arg(long)]
    pub(crate) json: bool,
}

#[derive(Args, Debug)]
pub(crate) struct CleanArgs {
    /// Remove without confirming.
    #[arg(long)]
    pub(crate) yes: bool,
    /// Emit the report as one JSON document.
    #[arg(long)]
    pub(crate) json: bool,
}
