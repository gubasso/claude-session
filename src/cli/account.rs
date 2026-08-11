//! Parse shape for account management.

use crate::domain::identifier::Identifier;
use clap::{ArgAction, Args, Subcommand};

#[derive(Args, Debug)]
pub(crate) struct AccountArgs {
    // Requested help is a result rather than a diagnostic
    // (`cli-surface.md#help`), so the parser's own help flag stays disabled
    // and the request is carried as a value the dispatcher answers on standard
    // output at exit `0`. The flag is global inside this namespace so
    // `account list --help` reaches the subcommand's own help rather than the
    // namespace's. The id is distinct from the root flag's on purpose: clap
    // propagates a global argument's value up into the parent matches as well
    // as down, so sharing an id would make `account --help` also set the
    // root's flag and answer with the composed wrapper help instead.
    /// Print this command's help.
    #[arg(
        id = "account_help",
        long = "help",
        short = 'h',
        action = ArgAction::SetTrue,
        global = true,
    )]
    pub(crate) help_flag: bool,
    // Optional only so `account --help` can parse. A bare `account` is still
    // malformed, and the dispatcher raises the `Usage` the parser used to
    // ([ADR-0052](../../docs/decisions/ADR-0052-require-an-explicit-subcommand.md)).
    #[command(subcommand)]
    pub(crate) command: Option<AccountCommand>,
}

#[derive(Subcommand, Debug)]
pub(crate) enum AccountCommand {
    /// Delegate native saved-login setup to the child.
    Login(LoginArgs),
    /// List locally discovered accounts.
    List(ListArgs),
}

#[derive(Args, Debug)]
pub(crate) struct LoginArgs {
    /// Account to log in; the selected account when omitted.
    pub(crate) name: Option<Identifier>,
    /// Emit the report as one JSON document.
    #[arg(long)]
    pub(crate) json: bool,
}

#[derive(Args, Debug)]
pub(crate) struct ListArgs {
    /// Emit the report as one JSON document.
    #[arg(long)]
    pub(crate) json: bool,
}
