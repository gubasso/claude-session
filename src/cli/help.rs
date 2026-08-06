//! Parse shape for the wrapper-owned `help` verb.

use clap::Args;

/// Arguments accepted by `help` in this slice.
#[derive(Debug, Args)]
pub(crate) struct HelpArgs {}
