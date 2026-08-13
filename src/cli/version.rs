//! Parse shape for the wrapper-owned `version` verb.

use clap::{ArgAction, Args};

/// Arguments accepted by `version`.
#[derive(Debug, Args)]
pub(crate) struct VersionArgs {
    // Distinct id for the reason recorded on `account`, and distinct from the
    // root's own `version_flag`: a shared id propagates up into the root matches
    // and answers with the composed wrapper surface instead of this verb's help.
    /// Print this command's help.
    #[arg(id = "version_help", long = "help", short = 'h', action = ArgAction::SetTrue)]
    pub(crate) help_flag: bool,
    /// Emit one stable JSON document.
    #[arg(long)]
    pub(crate) json: bool,
}
