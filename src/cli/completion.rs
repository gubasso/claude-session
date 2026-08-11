//! Parse shape for the wrapper-owned `completion` verb.

use clap::{ArgAction, Args};
use clap_complete::Shell;

/// Arguments accepted by `completion`.
#[derive(Debug, Args)]
pub(crate) struct CompletionArgs {
    // Requested help is a result rather than a diagnostic
    // (`cli-surface.md#help`), so the parser's own help flag stays disabled and
    // the request is carried as a value the dispatcher answers on standard
    // output at exit `0`. The id is distinct from the root flag's for the
    // reason recorded on `account`: clap propagates a global argument's value
    // up into the parent matches, and a shared id would answer this with the
    // composed wrapper help instead of the verb's own.
    /// Print this command's help.
    #[arg(id = "completion_help", long = "help", short = 'h', action = ArgAction::SetTrue)]
    pub(crate) help_flag: bool,
    // The shell set is `clap_complete`'s rather than this project's, so the
    // five stay the generator's fact and no local enum has to be revisited
    // when it grows one (`cli-surface.md#help`). Optional only so
    // `completion --help` can parse; a bare `completion` is still malformed and
    // the dispatcher raises the `Usage` the parser no longer can.
    /// The shell whose completion script to emit.
    #[arg(value_enum)]
    pub(crate) shell: Option<Shell>,
}
