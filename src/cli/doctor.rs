//! Parse shape for the wrapper-owned health report.

use clap::{ArgAction, Args};

/// Doctor output and policy flags.
#[allow(
    clippy::struct_excessive_bools,
    reason = "each field is one declared flag, which is the parse shape clap requires"
)]
#[derive(Args, Debug)]
pub(crate) struct DoctorArgs {
    // Distinct id for the reason recorded on `account`: clap propagates a
    // shared id up into the root matches, which would answer with the composed
    // wrapper help instead of this verb's own.
    /// Print this command's help.
    #[arg(id = "doctor_help", long = "help", short = 'h', action = ArgAction::SetTrue)]
    pub(crate) help_flag: bool,
    /// Emit one JSON document.
    #[arg(long)]
    pub(crate) json: bool,
    /// List catalog metadata without running probes.
    #[arg(long)]
    pub(crate) list: bool,
    /// Promote warnings to exit status one.
    #[arg(long)]
    pub(crate) strict: bool,
}
