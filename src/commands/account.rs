//! Account discovery and native-login orchestration.

use crate::{
    adapters::{filesystem::SystemFileSystem, process::ProcessRunner, terminal::Terminal},
    commands::dispatch::{DispatchOutcome, OutputMode},
    context::AppContext,
    domain::{
        account::TokenIngest,
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

/// Reports one account's mode, health, selection provenance, and warnings.
///
/// An inspection verb: it exits `0` whatever it finds, including an unusable
/// credential and a failed child probe. Those are answers, and the invocation
/// that tries to use the authentication is what fails.
pub(crate) fn status(
    context: &AppContext,
    name: Option<Identifier>,
) -> Result<DispatchOutcome, AppError> {
    use crate::services::account::status as service;
    let (account, selected) = service::subject(context, name)?;
    service::require_existing(context, &account)?;
    let status = service::project(context, &account, selected)?;
    crate::ui::account::status(
        context.writer(),
        context.output_mode() == OutputMode::Json,
        &status,
    )?;
    Ok(DispatchOutcome::Complete(0))
}

/// Removes one account's local state, after confirming.
///
/// Declining is not a failure: the verb stops before any side effect and exits
/// `0`, because nothing was removed is an outcome rather than an error.
pub(crate) fn remove(
    context: &AppContext,
    account: &Identifier,
    consented: bool,
) -> Result<DispatchOutcome, AppError> {
    use crate::services::account::{remove as service, status};
    status::require_existing(context, account)?;
    let is_selected = context.account_selection().account() == Some(account);
    if is_selected {
        crate::ui::account::selected_removal_warning(account.as_str());
    }
    if !consented && !confirm(context, account)? {
        let declined = crate::domain::account::Removal {
            account: account.clone(),
            path: context.paths().account(account),
            removed: false,
            mode: None,
            marker_cleared: None,
        };
        crate::ui::account::removal(
            context.writer(),
            context.output_mode() == OutputMode::Json,
            &declined,
        )?;
        return Ok(DispatchOutcome::Complete(0));
    }
    // The lock is moved straight in: it is released by the removal itself,
    // after the tree it guards is gone.
    let removal = service::perform(
        context,
        account,
        crate::services::account::hold(context, account)?,
    )?;
    crate::ui::account::removal(
        context.writer(),
        context.output_mode() == OutputMode::Json,
        &removal,
    )?;
    crate::ui::account::removal_consequences(context.writer(), &removal);
    Ok(DispatchOutcome::Complete(0))
}

/// Asks the one confirmation question, on the controlling terminal.
///
/// Standard input is deliberately not consulted: the prompt has to survive
/// `something | claude-session-rs account remove work`, and it has to be
/// invisible to `--json` consumers reading standard output.
fn confirm(context: &AppContext, account: &Identifier) -> Result<bool, AppError> {
    let unavailable = |why: String| {
        AppError::new(
            ErrorKind::Unavailable,
            Diagnostic::new(
                "account remove needs a controlling terminal to confirm",
                "/dev/tty",
                why,
                "pass --yes to remove without confirming",
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
    let prompt = format!(
        "Remove account '{}' and all of its local state? [y/N] ",
        account.as_str()
    );
    let answer = context
        .adapters()
        .terminal()
        .ask(&prompt)
        .map_err(|error| unavailable(error.to_string()))?;
    Ok(crate::domain::consent::decide(answer.as_deref()).granted())
}

/// Logs in, in whichever mode was requested.
///
/// Both modes share one wrapper: a first login that fails removes the account
/// directory it created, so neither leaves a half-made account behind. What
/// differs is entirely inside — one delegates a browser flow to the child and
/// commits nothing of its own, the other ingests a secret and commits a pair.
pub(crate) fn login(
    context: &AppContext,
    name: Option<Identifier>,
    token: Option<TokenIngest>,
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
    let result = match token {
        Some(request) => token_login(context, &account, &directory, &program, &request),
        None => native_login(context, &account, &directory, program),
    };
    if result.is_err() && !existed && !crate::services::account::committed(context, &account) {
        // Removal is part of the failed first login (accounts.md), so a
        // removal that did not happen is the fact the user has to act on:
        // reporting only the login failure would claim a transaction that
        // still has state behind it. The login failure is folded into the
        // diagnostic rather than dropped.
        //
        // `existed` was sampled before a child that can run for minutes, so it
        // cannot be the whole test: a second login against the same new account
        // may have committed in the meantime, and deleting its account because
        // this run failed would destroy a credential nobody asked to remove.
        // The committed check is what makes the cleanup about this run's own
        // incomplete state rather than about the directory's age.
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

/// Ingests, verifies, and commits a long-lived subscription token.
///
/// Verification precedes the commit and the commit takes the credential lock,
/// which is what makes a rotation safe against a concurrent one: a candidate
/// that does not work never replaces one that does.
fn token_login(
    context: &AppContext,
    account: &Identifier,
    directory: &std::path::Path,
    program: &std::path::Path,
    request: &TokenIngest,
) -> Result<DispatchOutcome, AppError> {
    use crate::services::account::token;
    // The configuration directory exists before the probe, because the probe
    // points the child at it. A failure from here on is a failed first login
    // and the caller removes what this created.
    crate::services::account::prepare_login(context, account)?;
    let candidate = token::ingest(context, account, program, request)?;
    let probe = token::probe(context, account, program, &candidate);
    if probe.status != crate::domain::account::ProbeStatus::Ok {
        return Err(token::verification_failure(probe));
    }
    let metadata = token::rotate(
        context,
        account,
        &candidate,
        token::recorded_at(context, request),
    )?;
    crate::ui::account::login(
        context.writer(),
        context.output_mode() == OutputMode::Json,
        account.as_str(),
        directory,
        &metadata,
    )?;
    Ok(DispatchOutcome::Complete(0))
}

#[allow(
    clippy::too_many_lines,
    reason = "the login transaction keeps cleanup and commit ordering visible"
)]
fn native_login(
    context: &AppContext,
    account: &Identifier,
    directory: &std::path::Path,
    program: std::path::PathBuf,
) -> Result<DispatchOutcome, AppError> {
    let account = account.clone();
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
    match (outcome, committable) {
        (Ok(ChildOutcome::Exited(0)), Ok(true)) => {
            crate::services::account::write_login_metadata(context, &account).and_then(|metadata| {
                crate::ui::account::login(
                    context.writer(),
                    context.output_mode() == OutputMode::Json,
                    account.as_str(),
                    directory,
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
                "retry claude-session-rs account login from an interactive terminal",
            );
            diagnostic.child_exit = child_exit;
            Err(AppError::new(ErrorKind::Auth, diagnostic))
        }
        (_, Err(error)) | (Err(error), Ok(_)) => Err(error),
    }
}
