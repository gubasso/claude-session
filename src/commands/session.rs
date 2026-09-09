//! Session liveness reporting and dead-session collection.

use crate::{
    adapters::terminal::Terminal as _,
    commands::dispatch::{DispatchOutcome, OutputMode},
    context::AppContext,
    domain::{registration::Subject, witness::Ground},
    error::{AppError, Diagnostic, ErrorKind},
    services::session::gc::{self, SessionFinding},
};

/// Reports every session directory and its liveness verdict.
///
/// An inspection verb: it exits `0` whatever it finds, including nothing.
/// The verdicts are answers, and `clean` is the invocation that acts on them.
pub(crate) fn list(
    context: &AppContext,
    name: Option<&Subject>,
) -> Result<DispatchOutcome, AppError> {
    let mut findings = gc::survey(context)?;
    if let Some(name) = name {
        findings.retain(|finding| finding.answers_to(name));
    }
    crate::ui::session::list(
        context.writer(),
        context.output_mode() == OutputMode::Json,
        context.color(),
        &findings,
        name,
    )?;
    Ok(DispatchOutcome::Complete(0))
}

/// Removes every session directory this run cannot prove is live, after
/// confirming.
///
/// Declining is not a failure: the verb stops before any side effect and exits
/// `0`, because nothing was removed is an outcome rather than an error
/// ([ADR-0112]).
///
/// [ADR-0112]: ../../docs/decisions/ADR-0112-keep-only-the-session-proven-live.md
pub(crate) fn clean(context: &AppContext, consented: bool) -> Result<DispatchOutcome, AppError> {
    let findings = gc::survey(context)?;
    // A run that cannot name its own namespace has placed no record at all, so
    // every directory would read as unaccounted for and the verb would take
    // the whole tree. Blindness is not evidence, and this is the one state
    // where refusing beats collecting ([ADR-0112]).
    if findings
        .iter()
        .any(|finding| finding.ground == Ground::Unplaced)
    {
        return Err(AppError::new(
            ErrorKind::Unavailable,
            Diagnostic::new(
                "session clean cannot judge this tree",
                "the namespace of this run",
                "this run cannot name the namespace its own session directories are scoped by"
                    .to_owned(),
                "see what each session is judged as with: claude-session session list",
            ),
        ));
    }
    let collectable: Vec<SessionFinding> = findings
        .into_iter()
        .filter(|finding| finding.verdict.collectable())
        .collect();
    let json = context.output_mode() == OutputMode::Json;
    if collectable.is_empty() {
        crate::ui::session::collection(context.writer(), json, context.color(), &[], 0, false)?;
        return Ok(DispatchOutcome::Complete(0));
    }
    if !consented && !confirm(context, &collectable)? {
        crate::ui::session::collection(context.writer(), json, context.color(), &[], 0, true)?;
        return Ok(DispatchOutcome::Complete(0));
    }
    let collection = gc::collect(context, &collectable)?;
    crate::ui::session::collection(
        context.writer(),
        json,
        context.color(),
        &collection.removed,
        collection.pruned_namespaces,
        false,
    )?;
    Ok(DispatchOutcome::Complete(0))
}

/// Asks the one confirmation question, on the controlling terminal.
///
/// Standard input is deliberately not consulted: the prompt has to survive
/// `something | claude-session session clean`, and it has to be invisible
/// to `--json` consumers reading standard output. The question's wording is
/// the renderer's, so this decides only whether it can be asked at all.
fn confirm(context: &AppContext, collectable: &[SessionFinding]) -> Result<bool, AppError> {
    let unavailable = |why: String| {
        AppError::new(
            ErrorKind::Unavailable,
            Diagnostic::new(
                "session clean needs a controlling terminal to confirm",
                "/dev/tty",
                why,
                "pass --yes to collect without confirming",
            ),
        )
    };
    match context.adapters().terminal().available() {
        Ok(true) => {}
        Ok(false) => {
            return Err(unavailable(
                "no controlling terminal is available".to_owned(),
            ));
        }
        Err(error) => return Err(unavailable(error.to_string())),
    }
    let answer = context
        .adapters()
        .terminal()
        .ask(&crate::ui::session::prompt(collectable))
        .map_err(|error| unavailable(error.to_string()))?;
    Ok(crate::domain::consent::decide(answer.as_deref()).granted())
}
