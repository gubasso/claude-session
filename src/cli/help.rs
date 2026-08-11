//! Parse shape for the wrapper-owned `help` verb.

use clap::{Args, ValueEnum};

/// Arguments accepted by `help` in this slice.
#[derive(Debug, Args)]
pub(crate) struct HelpArgs {
    /// The verb whose help to print, or the composed surface when absent.
    ///
    /// Only the verbs whose requested-help surface is implemented are listed,
    /// so an unimplemented one is a `Usage` diagnostic naming what does exist
    /// rather than a promise the wrapper cannot keep.
    pub(crate) verb: Option<HelpTopic>,
}

/// A verb with an implemented requested-help surface.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub(crate) enum HelpTopic {
    Account,
    Completion,
    Config,
    Man,
    Profile,
}
