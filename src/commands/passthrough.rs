//! Native passthrough orchestration.

use crate::{commands::dispatch::DispatchOutcome, context::AppContext, error::AppError};
use std::ffi::OsString;

/// Prepares the launch and hands it back for the entry point to become.
///
/// The order is the sequence [process runtime] specifies, and the child comes
/// first: a missing child or a refused recursion fails before the wrapper
/// creates an account directory or materialises a composed entry, so a run that
/// cannot launch leaves nothing behind for having tried.
///
/// Session preparation then happens only for what the run actually selected:
/// with neither an account nor a profile the wrapper touches no storage at all,
/// which is what an empty configuration tree launching the child unchanged
/// requires. A storage failure is the wrapper's own, so it fails before the
/// launch rather than being confused with a child status.
///
/// The exec itself is deliberately not performed here. It has to follow the log
/// flush, and the flush belongs to the entry point that owns the guard
/// ([ADR-0084](../../docs/decisions/ADR-0084-exec-the-child-instead-of-supervising-it.md)).
///
/// [process runtime]: ../../docs/reference/process-runtime.md#the-exec
pub(crate) fn run(
    context: &AppContext,
    arguments: Vec<OsString>,
) -> Result<DispatchOutcome, AppError> {
    let program = crate::services::child::program(context)?;
    let account = crate::services::account::validate_selected_launch(context)?;
    let mode = account.map(|account| account.mode);
    // The floor guards shared-login refresh coordination, which token mode does
    // not use: it injects a credential the wrapper stored rather than one the
    // child renews across processes.
    if mode == Some(crate::domain::account::AuthMode::Login) {
        crate::services::account::enforce_version_floor(context, &program)?;
    }
    // Before the exec, because after it there is no wrapper left to say
    // anything. Nothing here changes what the child receives; each line
    // describes a credential the user's own environment or configuration
    // directory supplies, which the wrapper never strips.
    if let (Some(selected), Some(mode)) = (context.session().account(), mode) {
        for warning in
            crate::services::account::launch_warnings(context, &selected.id, mode.report())
        {
            tracing::warn!("{}", warning.message());
        }
    }
    let entry = match context.session().profile() {
        Some(profile) => Some(crate::services::storage::entry::resolve(context, profile)?),
        None => None,
    };
    crate::services::account::write_marker(context)?;
    Ok(DispatchOutcome::Exec(crate::services::child::launch(
        context,
        program,
        entry.as_ref().map(|resolved| resolved.settings.as_path()),
        arguments,
        mode,
    )?))
}
