//! The child half of a composed read-only verb.
//!
//! This is not the wrapper's own output, and it is not the delimiter. It is the
//! section after the delimiter, which every claimed read-only spelling appends
//! identically.

use crate::{
    adapters::process::ProcessRunner, context::AppContext, domain::child::ChildInvocation,
    error::AppError, ui::writer::output_error,
};

/// Runs the child's own answer, or names in one line why it could not.
///
/// Resolution and spawn are the same condition to a reader: the section is
/// missing either way, so both are reported the same way. The section is part
/// of the result, so its absence does not change the exit status.
///
/// The resolution arrives already attempted, because a verb that reports the
/// resolved path above the delimiter has performed it, and resolving twice
/// would log the same operation twice.
pub(crate) fn child_section(
    context: &AppContext,
    resolved: Result<ChildInvocation, AppError>,
) -> Result<(), AppError> {
    let failure = match resolved {
        Ok(invocation) => context
            .adapters()
            .process()
            .run_inherited(&invocation)
            .err(),
        Err(error) => Some(error),
    };
    let Some(error) = failure else {
        return Ok(());
    };
    // The wrapper's own section already rendered, so this says why the child's
    // is missing rather than failing the verb. The kind token stays in the
    // diagnostic and the log, where a caller reads it; here a person is told
    // what happened ([ADR-0093]).
    context
        .writer()
        .stdout(
            crate::ui::prose::paragraph(&format!(
                "claude could not be run, so its own report is missing: {}.",
                error.diagnostic().why
            ))
            .as_bytes(),
        )
        .map_err(|output| output_error(&output))
}
