//! Native passthrough orchestration.

use crate::{
    adapters::process::ProcessRunner, commands::dispatch::DispatchOutcome, context::AppContext,
    domain::child::ChildOutcome, error::AppError,
};
use std::ffi::OsString;

/// Launches the child with opaque user arguments.
///
/// Session preparation happens here, before the child is resolved, and only for
/// what the run actually selected: with neither an account nor a profile the
/// wrapper touches no storage at all, which is what an empty configuration tree
/// launching the child unchanged requires. A storage failure is the wrapper's
/// own, so it fails before the spawn rather than being confused with a child
/// status.
pub(crate) fn run(
    context: &AppContext,
    arguments: Vec<OsString>,
) -> Result<DispatchOutcome, AppError> {
    crate::services::session::prepare_account(context)?;
    if let Some(profile) = context.session().profile() {
        // Resolved, not passed: prefixing `--settings` onto the child's
        // argument vector belongs with the supervised runtime.
        let _ = crate::services::storage::entry::resolve(context, profile)?;
    }
    let invocation = crate::services::child::invocation(context, arguments)?;
    match context.adapters().process().run_inherited(&invocation)? {
        ChildOutcome::Exited(code) => Ok(DispatchOutcome::Complete(code)),
        ChildOutcome::Signaled(signal) => Ok(DispatchOutcome::Signaled(signal)),
    }
}
