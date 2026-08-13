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
        context.color(),
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
        context.color(),
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
            context.color(),
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
        context.color(),
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
    // The question a person is answering, with what it costs them stated
    // before they answer it rather than after.
    let prompt = format!(
        concat!(
            "This deletes everything this wrapper stored for \"{}\", and revokes",
            " nothing at the provider.\nRemove it? [y/N] "
        ),
        account.as_str()
    );
    let answer = context
        .adapters()
        .terminal()
        .ask(&prompt)
        .map_err(|error| unavailable(error.to_string()))?;
    Ok(crate::domain::consent::decide(answer.as_deref()).granted())
}

/// Binds one account to the profile it runs with.
///
/// Its own verb rather than a re-login, because changing which settings an
/// account uses should not cost a browser flow or a pasted token
/// ([ADR-0097](../../docs/decisions/ADR-0097-rebind-a-profile-without-re-authenticating.md)).
pub(crate) fn bind(
    context: &AppContext,
    account: &Identifier,
    profile: &Identifier,
) -> Result<DispatchOutcome, AppError> {
    let binding = crate::services::account::bind::perform(context, account, profile)?;
    crate::ui::account::binding(
        context.writer(),
        context.output_mode() == OutputMode::Json,
        context.color(),
        account,
        &binding,
    )?;
    Ok(DispatchOutcome::Complete(0))
}

/// Resolves the profile a login binds the account to, and where it came from.
///
/// Three rungs, refusing rather than guessing at the end of them: an account
/// created without a profile is the state this slice exists to remove, so the
/// refusal names both ways to supply one instead of leaving the choice implicit.
fn login_profile(
    context: &AppContext,
    account: &Identifier,
    requested: Option<Identifier>,
) -> Result<(Identifier, crate::domain::config::Source), AppError> {
    use crate::domain::config::Source;
    if let Some(profile) = requested {
        return Ok((profile, Source::Cli));
    }
    if let Some(binding) = crate::services::account::bind::read(context.paths(), account)? {
        return Ok((binding.profile, Source::Account));
    }
    context.config().profile().cloned().map_or_else(
        || {
            Err(AppError::new(
                ErrorKind::Config,
                Diagnostic::new(
                    "account login needs a profile",
                    "account login",
                    concat!(
                        "no profile was given, this account has no binding, and no",
                        " default_profile is configured"
                    ),
                    concat!(
                        "pass --profile <name>, or set default_profile in your",
                        " configuration file"
                    ),
                ),
            ))
        },
        |profile| Ok((profile, context.config().profile_source())),
    )
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
    profile: Option<Identifier>,
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
    // Before the child and before any directory is made: a login that cannot
    // say which profile the account runs with is refused while refusing is
    // still free.
    let profile = login_profile(context, &account, profile)?;
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
        Some(request) => token_login(context, &account, &directory, &program, &request, &profile),
        None => native_login(context, &account, &directory, program, &profile),
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
    profile: &(Identifier, crate::domain::config::Source),
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
    let binding = commit_binding(context, account, profile)?;
    crate::ui::account::login(
        context.writer(),
        context.output_mode() == OutputMode::Json,
        account.as_str(),
        directory,
        &metadata,
        &binding,
        context.color(),
    )?;
    Ok(DispatchOutcome::Complete(0))
}

/// Records the profile this login bound, and how the report should say so.
///
/// Written after the authentication commits, so a failed login never leaves a
/// binding behind for an account that does not exist, and never replaces a
/// working account's binding with one whose login went on to fail.
///
/// That order has one cost, and the diagnostic below is what pays it: the
/// credential is already durable when this runs, so a failure here leaves an
/// account that authenticated and did not record its profile. Removing the
/// account over it would destroy a credential the user just proved, and the
/// state is one the wrapper already reports and already repairs — an unbound
/// account. So the refusal says what survived and names the verb that finishes
/// the job, rather than reporting a bare write failure whose next action would
/// send the reader to the wrong place.
fn commit_binding(
    context: &AppContext,
    account: &Identifier,
    profile: &(Identifier, crate::domain::config::Source),
) -> Result<crate::ui::account::Binding, AppError> {
    use crate::services::account::bind;
    let (name, source) = profile;
    let binding = crate::domain::account::ProfileBinding {
        profile: name.clone(),
        recorded_at: crate::domain::account::RecordedAt::new_unchecked(
            crate::adapters::clock::rfc3339_utc(crate::adapters::clock::Clock::now(
                &context.adapters().clock(),
            )),
        ),
    };
    // Under the account lock, as every other write to an account is: a login
    // racing a rebind must not interleave the two, or the account ends up
    // authenticated by one and bound by the other.
    let write = crate::services::account::hold(context, account)
        .and_then(|_lock| bind::write(context, account, &binding));
    if let Err(error) = write {
        let mut diagnostic = error.diagnostic().clone();
        "account authenticated but its profile was not recorded".clone_into(&mut diagnostic.what);
        diagnostic.why = format!(
            concat!(
                "{}. The credential for \"{}\" is stored and usable, so this login",
                " is not being undone; only the profile binding is missing"
            ),
            diagnostic.why.trim_end_matches('.'),
            account.as_str()
        );
        diagnostic.hint = format!(
            "run claude-session-rs account bind {} --profile {}",
            account.as_str(),
            name.as_str()
        );
        return Err(AppError::new(error.kind(), diagnostic));
    }
    Ok(crate::ui::account::Binding {
        profile: name.clone(),
        source: *source,
        present: bind::profile_present(context, name),
    })
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
    profile: &(Identifier, crate::domain::config::Source),
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
                let binding = commit_binding(context, &account, profile)?;
                crate::ui::account::login(
                    context.writer(),
                    context.output_mode() == OutputMode::Json,
                    account.as_str(),
                    directory,
                    &metadata,
                    &binding,
                    context.color(),
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
