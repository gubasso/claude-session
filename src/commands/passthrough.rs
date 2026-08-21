//! Native passthrough orchestration.

use crate::{
    adapters::environment::Environment as _,
    commands::dispatch::DispatchOutcome,
    context::AppContext,
    error::{AppError, Diagnostic, ErrorKind},
};
use std::ffi::OsString;

/// Prepares the launch and hands it back for the entry point to become.
///
/// The order is the sequence [process runtime] specifies, and the child comes
/// first: a missing child or a refused recursion fails before the wrapper
/// creates an account directory or materialises a composed entry, so a run that
/// cannot launch leaves nothing behind for having tried.
///
/// Binding is verified immediately after child resolution and before account
/// validation, profile composition, marker writes, or launch construction.
/// This preserves child-resolution priority while leaving an unbound refusal
/// free of session side effects ([ADR-0090], ADR-0091): no account directory,
/// no composed entry, and no marker write. The log sink is installed for every
/// invocation before dispatch, so it is not one of them. A later storage
/// failure is the
/// wrapper's own, so it fails before launch rather than being confused with a
/// child status.
///
/// The exec itself is deliberately not performed here. It has to follow the log
/// flush, and the flush belongs to the entry point that owns the guard
/// ([ADR-0084](../../docs/decisions/ADR-0084-exec-the-child-instead-of-supervising-it.md)).
///
/// [process runtime]: ../../docs/reference/process-runtime.md#the-exec
/// [ADR-0090]: ../../docs/decisions/ADR-0090-require-account-and-profile-before-child-launch.md
pub(crate) fn run(
    context: &AppContext,
    arguments: Vec<OsString>,
) -> Result<DispatchOutcome, AppError> {
    let program = crate::services::child::program(context)?;
    validate_binding(context)?;
    let account = crate::services::account::validate_selected_launch(context)?;
    let mode = account.as_ref().map(|account| account.mode);
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
        // Read here, written below. This run reports what it is about to meet
        // in the session directory as it stands, before the seed answers it;
        // a launch that would work is never refused over the answer
        // ([ADR-0105]).
        //
        // [ADR-0105]: ../../docs/decisions/ADR-0105-seed-a-session-at-launch.md
        if !crate::services::account::onboarding::readiness(context, &selected.id).ready() {
            tracing::warn!(
                "{}",
                crate::domain::account::Warning::FirstRunOnboarding.message()
            );
        }
        // Token mode only, and never a refusal: without a plan the child still
        // launches and still signs in, it just describes the session as an API
        // one and picks the model it defaults to without one
        // ([ADR-0099](../../docs/decisions/ADR-0099-declare-the-plan-a-token-cannot-carry.md)).
        if mode == crate::domain::account::AuthMode::Token
            && account
                .as_ref()
                .is_some_and(|account| account.plan.is_none())
        {
            tracing::warn!(
                "{}",
                crate::domain::account::Warning::PlanUndeclared.message()
            );
        }
    }
    let entry = match context.session().profile() {
        Some(profile) => Some(crate::services::storage::entry::resolve(context, profile)?),
        None => None,
    };
    // The session directory is this run's, so it is built here rather than at
    // login: no terminal is knowable before one launches ([ADR-0105]). The
    // seed follows the directory it writes into, and the trust answer follows
    // the seed, because both are questions the directory's newness makes the
    // child ask.
    //
    // [ADR-0105]: ../../docs/decisions/ADR-0105-seed-a-session-at-launch.md
    let selected = context.session().account().ok_or_else(|| {
        AppError::new(
            ErrorKind::Internal,
            Diagnostic::new(
                "a bound launch reached the session step with no account",
                "the launch sequence",
                "the binding gate above should have refused this run",
                "report this invariant",
            ),
        )
    })?;
    let agent = crate::services::session::agent(context)?;
    let session = crate::services::session::materialise(context, &selected.id, &agent)?;
    crate::services::assets::supply(context, &session)?;
    let trust = context
        .config()
        .auto_trust_cwd()
        .then(|| context.environment().current_dir());
    crate::services::account::hold(context, &selected.id).and_then(|_lock| {
        crate::services::account::onboarding::make_launchable(context, &selected.id, trust)
    })?;
    crate::services::account::write_marker(context)?;
    Ok(DispatchOutcome::Exec(crate::services::child::launch(
        context,
        program,
        entry.as_ref().map(|resolved| resolved.settings.as_path()),
        arguments,
        account.as_ref(),
        &session,
    )?))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MissingBinding {
    Account,
    Profile,
    AccountAndProfile,
}

const SELECT_EACH: &str = concat!(
    "then select each with --account and --profile or with ",
    "default_account and default_profile"
);

fn validate_binding(context: &AppContext) -> Result<(), AppError> {
    let session = context.session();
    let missing = match (session.account(), session.profile()) {
        (Some(_), Some(_)) => return Ok(()),
        (None, Some(_)) => MissingBinding::Account,
        (Some(_), None) => MissingBinding::Profile,
        (None, None) => MissingBinding::AccountAndProfile,
    };
    // The subject the diagnostic renders as a sentence, so the half that did
    // resolve is named and the half that did not is said in words. A pair of
    // `key=value` tokens carried the same two facts and read as a field dump
    // at a person, which is the shape [ADR-0093] removed from every surface
    // the wrapper writes outside `--json`.
    let subject = match missing {
        MissingBinding::Account => format!(
            "this launch, which resolved the \"{}\" profile but no account",
            session.profile().map_or("", |selected| selected.as_str())
        ),
        MissingBinding::Profile => format!(
            "this launch, which resolved the \"{}\" account but no profile",
            session
                .account()
                .map_or("", |selected| selected.id.as_str())
        ),
        MissingBinding::AccountAndProfile => {
            "this launch, which resolved neither an account nor a profile".to_owned()
        }
    };
    // The hint names the wrapper's own surfaces and the directory it reads,
    // never a repository path: the published crate excludes `docs/`, so a
    // path that exists in the checkout does not exist for an installed
    // binary ([ADR-0091]).
    let profiles = context.paths().profiles();
    let login = "run claude-session account login <name>";
    let author = format!("write a profile at {}/<name>.yaml", profiles.display());
    let (why, hint) = match missing {
        MissingBinding::Account => (
            "a child launch requires a resolved account",
            format!("{login}, then select it with --account or default_account"),
        ),
        MissingBinding::Profile => (
            "a child launch requires a resolved profile",
            format!(
                "{author}, then bind it with claude-session account bind \
                <account> --profile <name>, or select it with --profile or \
                default_profile"
            ),
        ),
        MissingBinding::AccountAndProfile => (
            "a child launch requires a resolved account and profile",
            format!("{login} and {author}, {SELECT_EACH}"),
        ),
    };
    Err(AppError::new(
        ErrorKind::Config,
        Diagnostic::new(
            "child launch is not bound to a complete session",
            subject,
            why,
            hint,
        ),
    ))
}
