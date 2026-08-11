//! Parse shape for the wrapper-owned `man` verb.

use clap::{ArgAction, Args};

/// Arguments accepted by `man`.
///
/// The verb takes no `--json`, because roff is not data
/// ([ADR-0086](../../docs/decisions/ADR-0086-emit-one-man-page-to-standard-output.md)),
/// and no output destination, because the page goes to standard output alone.
#[derive(Debug, Args)]
pub(crate) struct ManArgs {
    // Distinct id for the reason recorded on `account`: a shared help id
    // propagates up into the root matches and answers with the composed
    // wrapper help instead of this verb's own.
    /// Print this command's help.
    #[arg(id = "man_help", long = "help", short = 'h', action = ArgAction::SetTrue)]
    pub(crate) help_flag: bool,
}
