//! Parse shape for the wrapper-owned `version` verb.

use clap::Args;

/// Arguments accepted by `version`.
#[derive(Debug, Args)]
pub(crate) struct VersionArgs {
    /// Emit one stable JSON document.
    #[arg(long)]
    pub(crate) json: bool,
}
