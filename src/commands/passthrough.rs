//! Native passthrough orchestration.

use crate::{
    adapters::process::ProcessRunner, commands::dispatch::DispatchOutcome, context::AppContext,
    domain::child::ChildOutcome, error::AppError,
};
use std::ffi::OsString;

/// Launches the child with opaque user arguments.
pub(crate) fn run(
    context: &AppContext,
    arguments: Vec<OsString>,
) -> Result<DispatchOutcome, AppError> {
    let invocation = crate::services::child::invocation(context, arguments)?;
    match context.adapters().process().run_inherited(&invocation)? {
        ChildOutcome::Exited(code) => Ok(DispatchOutcome::Complete(code)),
        ChildOutcome::Signaled(signal) => Ok(DispatchOutcome::Signaled(signal)),
    }
}
