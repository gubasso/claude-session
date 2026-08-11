//! Parse shape for the wrapper-owned health report.

use clap::Args;

/// Doctor output and policy flags.
#[derive(Args, Debug)]
pub(crate) struct DoctorArgs {
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
