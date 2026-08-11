//! Account discovery and native-login orchestration.

use crate::{
    adapters::{filesystem::SystemFileSystem, process::ProcessRunner, terminal::Terminal},
    commands::dispatch::{DispatchOutcome, OutputMode},
    context::AppContext,
    domain::{
        child::{ChildInvocation, ChildOutcome},
        identifier::Identifier,
    },
    error::{AppError, Diagnostic, ErrorKind},
};

pub(crate) fn list(context: &AppContext) -> Result<DispatchOutcome, AppError> {
    let accounts = crate::services::account::discover(context)?;
    crate::ui::account::list(
        context.writer(),
        context.output_mode() == OutputMode::Json,
        context.account_selection().source(),
        &accounts,
    )?;
    Ok(DispatchOutcome::Complete(0))
}

#[allow(
    clippy::too_many_lines,
    reason = "the login transaction keeps cleanup and commit ordering visible"
)]
pub(crate) fn login(
    context: &AppContext,
    name: Option<Identifier>,
) -> Result<DispatchOutcome, AppError> {
    let account = name
        .or_else(|| context.account_selection().account().cloned())
        .ok_or_else(|| {
            AppError::new(
                ErrorKind::Usage,
                Diagnostic::new(
                    "account login needs an account",
                    "account login",
                    "no name or selected account was available",
                    "pass an account name or select one with --account",
                ),
            )
        })?;
    let program = crate::services::child::program(context)?;
    match context.adapters().terminal().available() {
        Ok(true) => {}
        Ok(false) => {
            return Err(AppError::new(
                ErrorKind::Unavailable,
                Diagnostic::new(
                    "native login needs a controlling terminal",
                    "/dev/tty",
                    "no controlling terminal is available",
                    "run account login from an interactive terminal",
                ),
            ));
        }
        Err(error) => {
            return Err(AppError::new(
                ErrorKind::Unavailable,
                Diagnostic::new(
                    "native login needs a controlling terminal",
                    "/dev/tty",
                    error.to_string(),
                    "run account login from an interactive terminal",
                ),
            ));
        }
    }
    let directory = context.paths().account(&account);
    let existed = SystemFileSystem::look(&directory)
        .map_err(|error| {
            AppError::new(
                ErrorKind::Io,
                Diagnostic::new(
                    "account path could not be inspected",
                    directory.display().to_string(),
                    error.to_string(),
                    "check the account path",
                ),
            )
        })?
        .is_some();
    crate::services::account::prepare_login(context, &account)?;
    let mut environment = crate::services::child::subroutine_environment(context);
    environment.retain(|(key, _)| key != "CLAUDE_CONFIG_DIR");
    environment.push((
        "CLAUDE_CONFIG_DIR".into(),
        context.paths().account_config(&account).into_os_string(),
    ));
    let invocation =
        ChildInvocation::new(program, vec!["auth".into(), "login".into()], environment);
    let outcome = if context.output_mode() == OutputMode::Json {
        context
            .adapters()
            .process()
            .run_stdout_to_stderr(&invocation)
    } else {
        context.adapters().process().run_inherited(&invocation)
    };
    // A storage defect that appeared while the child ran is that defect, not a
    // login failure, so it keeps its own kind instead of being folded into the
    // "no safe saved-login path" branch below.
    let committable = match outcome {
        Ok(ChildOutcome::Exited(0)) => {
            crate::services::account::credentials_committable(context, &account)
        }
        _ => Ok(false),
    };
    let result = match (outcome, committable) {
        (Ok(ChildOutcome::Exited(0)), Ok(true)) => {
            crate::services::account::write_login_metadata(context, &account).and_then(|metadata| {
                crate::ui::account::login(
                    context.writer(),
                    context.output_mode() == OutputMode::Json,
                    account.as_str(),
                    &directory,
                    &metadata,
                )?;
                Ok(DispatchOutcome::Complete(0))
            })
        }
        (Ok(outcome), Ok(_)) => {
            let child_exit = match outcome {
                ChildOutcome::Exited(code) => Some(code),
                ChildOutcome::Signaled(_) => None,
            };
            let why = if matches!(outcome, ChildOutcome::Exited(0)) {
                "the child exited successfully but did not leave a safe saved-login path".into()
            } else {
                format!("claude auth login returned {outcome:?}")
            };
            let mut diagnostic = Diagnostic::new(
                "native child login failed",
                "claude auth login",
                why,
                "retry claude-session account login from an interactive terminal",
            );
            diagnostic.child_exit = child_exit;
            Err(AppError::new(ErrorKind::Auth, diagnostic))
        }
        (_, Err(error)) | (Err(error), Ok(_)) => Err(error),
    };
    if result.is_err() && !existed {
        // Removal is part of the failed first login (accounts.md), so a
        // removal that did not happen is the fact the user has to act on:
        // reporting only the login failure would claim a transaction that
        // still has state behind it. The login failure is folded into the
        // diagnostic rather than dropped.
        if let Err(cleanup) = crate::services::account::remove_incomplete(&directory) {
            let login = result.err().map_or_else(String::new, |error| {
                format!(" after {}", error.diagnostic().why)
            });
            let mut diagnostic = cleanup.diagnostic().clone();
            diagnostic.why = format!("{}{login}", diagnostic.why);
            return Err(AppError::new(cleanup.kind(), diagnostic));
        }
    }
    result
}
