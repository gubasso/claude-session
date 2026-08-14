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
    /// Delegate native saved-login setup to the child, or store a token.
    Login(LoginArgs),
    /// List locally discovered accounts.
    List(ListArgs),
    /// Report one account's mode, health, and selection provenance.
    Status(StatusArgs),
    /// Remove one account's local state.
    Remove(RemoveArgs),
    /// Bind one account to the profile it runs with.
    Bind(BindArgs),
}

#[derive(Args, Debug)]
pub(crate) struct BindArgs {
    // Required, like removal's: rebinding "whichever account was used last"
    // changes durable state under a name nobody typed.
    /// Account to bind.
    pub(crate) name: Identifier,
    /// Profile the account runs with.
    #[arg(long, value_name = "NAME")]
    pub(crate) profile: Identifier,
    /// Emit the report as one JSON document.
    #[arg(long)]
    pub(crate) json: bool,
}

// The `secret` group holds the two selectors that bring their own credential.
// It is what makes `--token --refresh-token` a parse error rather than a
// precedence question, and what lets `--stdin` say "whichever secret was asked
// for" instead of naming one of them.
// Four independent presence flags rather than a state the wrapper models: each
// one is on the command line or is not, the parser owns which combinations are
// legal, and folding them into an enum here would move that arbitration off the
// node that already renders its own usage.
#[allow(
    clippy::struct_excessive_bools,
    reason = "each bool is one command-line flag, and clap owns their exclusivity"
)]
#[derive(Args, Debug)]
#[command(group = clap::ArgGroup::new("secret").multiple(false))]
pub(crate) struct LoginArgs {
    /// Account to log in; the selected account when omitted.
    pub(crate) name: Option<Identifier>,
    /// Store a long-lived subscription token instead of a native saved login.
    #[arg(long, group = "secret")]
    pub(crate) token: bool,
    // Takes no value, for the reason `--token` takes none: a credential never
    // enters through argv
    // ([ADR-0027](../../docs/decisions/ADR-0027-ingest-secrets-only-from-stdin-or-a-terminal.md)).
    /// Exchange a claude.ai refresh token for a saved login, without a browser.
    #[arg(long = "refresh-token", group = "secret")]
    pub(crate) refresh_token: bool,
    // Not `requires`-gated on anything: an account's profile is orthogonal to
    // how it authenticates, and login is where the choice is made explicit
    // ([ADR-0096](../../docs/decisions/ADR-0096-bind-a-profile-to-an-account.md)).
    /// Profile the account runs with; `default_profile` when omitted.
    #[arg(long, value_name = "NAME")]
    pub(crate) profile: Option<Identifier>,
    // The token-only flags require `--token` rather than being silently
    // ignored without it, because each one alone reads as a request the wrapper
    // would then not honour. `--stdin` requires the group instead of one
    // member, because both secrets are read the same two ways.
    /// Read the secret from standard input instead of prompting.
    #[arg(long, requires = "secret")]
    pub(crate) stdin: bool,
    // Bound to `--refresh-token` alone: a saved login and a stored token are
    // each issued their own scopes by a flow this option does not take part in,
    // so offering it beside either would name a knob that changes nothing.
    /// The scopes the refresh token was issued with; a login's own when omitted.
    #[arg(long, requires = "refresh_token", value_name = "SCOPES")]
    pub(crate) scopes: Option<String>,
    /// The time the token was minted, when it was not minted just now.
    #[arg(long, requires = "token", value_name = "RFC3339")]
    pub(crate) minted_at: Option<String>,
    // Token-only for the same reason, and with a second one behind it: a saved
    // login carries its own plan to the child, so accepting this beside the
    // native flow would offer to answer a question that is not being asked
    // ([ADR-0099](../../docs/decisions/ADR-0099-declare-the-plan-a-token-cannot-carry.md)).
    /// The subscription plan the token belongs to; prompted for when omitted.
    #[arg(long, requires = "token", value_name = "PLAN")]
    pub(crate) plan: Option<String>,
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

#[derive(Args, Debug)]
pub(crate) struct StatusArgs {
    /// Account to report; the selected account when omitted.
    pub(crate) name: Option<Identifier>,
    /// Emit the report as one JSON document.
    #[arg(long)]
    pub(crate) json: bool,
}

#[derive(Args, Debug)]
pub(crate) struct RemoveArgs {
    // Required, unlike every other account positional. Removal is the one verb
    // whose subject cannot be inferred from the selection ladder: deleting
    // "whichever account was used last" is not something a user asks for.
    /// Account to remove.
    pub(crate) name: Identifier,
    /// Remove without confirming.
    #[arg(long)]
    pub(crate) yes: bool,
    /// Emit the report as one JSON document.
    #[arg(long)]
    pub(crate) json: bool,
}
