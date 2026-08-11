//! Parse shape for the wrapper-owned configuration report.
//!
//! This module is `crate::cli::config`, not `crate::config`: the parse shape of
//! the verb, not the configuration it reports on.

use clap::{ArgAction, Args};

/// Configuration report output flags.
#[derive(Args, Debug)]
pub(crate) struct ConfigArgs {
    /// Emit the report as one JSON document.
    #[arg(long)]
    pub(crate) json: bool,
    // A distinct id, for the same reason as every other verb's: clap propagates
    // a global argument's value into the parent match, and a shared `help` id
    // would answer with the composed wrapper help instead of this verb's.
    /// Print this command's help.
    #[arg(id = "config_help", long = "help", short = 'h', action = ArgAction::SetTrue)]
    pub(crate) help_flag: bool,
}
