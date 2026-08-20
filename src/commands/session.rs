//! Session liveness reporting and dead-session collection.

use crate::{
    adapters::terminal::Terminal as _,
    commands::dispatch::{DispatchOutcome, OutputMode},
    context::AppContext,
    domain::witness::Verdict,
    error::{AppError, Diagnostic, ErrorKind},
    services::session::gc::{self, SessionFinding},
};

/// Reports every session directory and its liveness verdict.
///
/// An inspection verb: it exits `0` whatever it finds, including nothing.
/// The verdicts are answers, and `clean` is the invocation that acts on them.
pub(crate) fn list(context: &AppContext) -> Result<DispatchOutcome, AppError> {
    let findings = gc::survey(context)?;
    crate::ui::session::list(
        context.writer(),
        context.output_mode() == OutputMode::Json,
        context.color(),
        &findings,
    )?;
    Ok(DispatchOutcome::Complete(0))
}

/// Removes every provably dead session directory, after confirming.
///
/// Declining is not a failure: the verb stops before any side effect and exits
/// `0`, because nothing was removed is an outcome rather than an error
/// ([ADR-0111]).
///
/// [ADR-0111]: ../../docs/decisions/ADR-0111-collect-only-the-provably-dead-session.md
pub(crate) fn clean(context: &AppContext, consented: bool) -> Result<DispatchOutcome, AppError> {
    let findings = gc::survey(context)?;
    let dead: Vec<SessionFinding> = findings
        .into_iter()
        .filter(|finding| matches!(finding.verdict, Verdict::Dead))
        .collect();
    let json = context.output_mode() == OutputMode::Json;
    if dead.is_empty() {
        crate::ui::session::collection(context.writer(), json, context.color(), &[], 0, false)?;
        return Ok(DispatchOutcome::Complete(0));
    }
    if !consented && !confirm(context, &dead)? {
        crate::ui::session::collection(context.writer(), json, context.color(), &[], 0, true)?;
        return Ok(DispatchOutcome::Complete(0));
    }
    let collection = gc::collect(context, &dead)?;
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
/// `something | claude-session-rs session clean`, and it has to be invisible
/// to `--json` consumers reading standard output. The preview is part of the
/// question, so what it costs is stated before it is answered.
fn confirm(context: &AppContext, dead: &[SessionFinding]) -> Result<bool, AppError> {
    use std::fmt::Write as _;
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
    let mut prompt = format!(
        "This deletes {} dead session {}, including the child state and history inside:\n",
        dead.len(),
        crate::ui::prose::plural(dead.len(), "directory", "directories")
    );
    for finding in dead {
        // Writing into a `String` cannot fail.
        let _ = writeln!(prompt, "  {}", finding.path.display());
    }
    prompt.push_str("Remove them? [y/N] ");
    let answer = context
        .adapters()
        .terminal()
        .ask(&prompt)
        .map_err(|error| unavailable(error.to_string()))?;
    Ok(crate::domain::consent::decide(answer.as_deref()).granted())
}
