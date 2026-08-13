//! The profile listing verb.

use crate::{
    commands::dispatch::{DispatchOutcome, OutputMode},
    context::AppContext,
    error::AppError,
};

/// Reports the profiles available to `--profile`.
///
/// An inspection verb: it exits `0` whatever it finds, including nothing at all.
/// An empty configuration directory is an answer rather than a failure.
pub(crate) fn list(context: &AppContext) -> Result<DispatchOutcome, AppError> {
    let profiles = crate::services::profile::discover(context)?;
    crate::ui::profile::list(
        context.writer(),
        context.output_mode() == OutputMode::Json,
        context.color(),
        context.config().profile_source(),
        &profiles,
    )?;
    Ok(DispatchOutcome::Complete(0))
}
