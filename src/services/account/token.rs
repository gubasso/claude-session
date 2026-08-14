//! Token-mode ingest, verification, commit, and read-back.
//!
//! The order here is the ingest sequence in `docs/reference/accounts.md`, and
//! two of its properties are structural rather than incidental. The candidate
//! reaches the child through the child's environment and never through argv, so
//! it cannot appear in a process listing. And verification happens before the
//! credential lock is taken, so the lock is never held across a child spawn.

use std::{ffi::OsString, io::Read as _, path::Path};

use crate::{
    adapters::{
        clock::{Clock, rfc3339_utc},
        filesystem::SystemFileSystem,
        process::ProcessRunner,
        terminal::Terminal,
    },
    context::AppContext,
    domain::{
        account::{
            AuthMode, AuthModeMetadata, ModeCommit, Plan, Probe, ProbeStatus, RecordedAt,
            TokenIngest, TokenSource,
        },
        child::{ChildInvocation, ChildOutcome},
        identifier::Identifier,
        secret::{Fingerprint, Secret},
    },
    error::{AppError, Diagnostic, ErrorKind},
    services::storage::{atomic, guard},
};

// Reached through `super` rather than imported, because this module's own
// public entry point is also called `ingest` and a bare name would be two
// things at one call site.
use super::ingest as source;

/// The estimated lifetime of a long-lived subscription token.
///
/// An estimate rather than a reading: the wrapper does not parse the token, and
/// the child warns about an approaching saved-login expiry but gives no
/// equivalent notice for an injected one, which simply stops working. Derived
/// from the mint time, it is the best a wrapper that refuses to inspect a
/// credential can offer, and every surface labels it as an estimate.
pub(crate) const ESTIMATED_LIFETIME_DAYS: u64 = 365;

/// The credential this module reads, for the shared ingest's wording.
const SUBJECT: source::Subject = source::Subject::Token;

/// Obtains one token candidate from a terminal or from standard input.
pub(crate) fn ingest(
    context: &AppContext,
    account: &Identifier,
    program: &Path,
    request: &TokenIngest,
) -> Result<Secret, AppError> {
    match request.source {
        TokenSource::Stdin => source::read_stdin(SUBJECT),
        TokenSource::Terminal => {
            source::require_terminal(context, SUBJECT)?;
            // Inherited streams, so the token the child prints goes to the
            // user's terminal and never through a pipe this process reads. The
            // wrapper then asks for it back, which is what keeps the value out
            // of any buffer the wrapper owns until the user chooses to paste
            // it.
            run_setup_token(context, account, program)?;
            source::read_terminal(
                context,
                SUBJECT,
                "Paste the token, then press Enter (it is not echoed): ",
            )
        }
    }
}

/// How many times a mistyped plan is worth asking about again.
///
/// Bounded rather than open, and never fatal: this runs after a credential has
/// been ingested and proven, so refusing here would throw away a token the user
/// may have to mint again. Running out of attempts records no plan, which is a
/// state every surface already reports and one more login repairs.
const PLAN_ATTEMPTS: usize = 3;

/// Obtains the plan to record beside the token.
///
/// The command line wins when it spoke, because a login that was told the
/// answer has no question to ask. Otherwise a terminal login asks, and a
/// `--stdin` login records none: standard input is carrying the credential, and
/// there is no second stream to hold an answer.
///
/// An empty line is a decline rather than a refusal. The plan makes a launch
/// describe itself correctly; it is not what makes the account work, so nothing
/// here may stand between a proven credential and its commit
/// ([ADR-0099](../../../docs/decisions/ADR-0099-declare-the-plan-a-token-cannot-carry.md)).
pub(crate) fn declare_plan(context: &AppContext, request: &TokenIngest) -> Option<Plan> {
    if request.plan.is_some() {
        return request.plan.clone();
    }
    if request.source == TokenSource::Stdin {
        return None;
    }
    let terminal = context.adapters().terminal();
    let mut prompt = plan_question();
    for _ in 0..PLAN_ATTEMPTS {
        let Ok(Some(answer)) = terminal.ask(&prompt) else {
            return None;
        };
        let answer = answer.trim();
        if answer.is_empty() {
            return None;
        }
        // A number is an ordinal into the offer above, which is the whole
        // reason the offer is numbered. Anything else is taken as typed, so a
        // plan the offer does not list is still reachable.
        let typed = answer
            .parse::<usize>()
            .ok()
            .and_then(|index| index.checked_sub(1))
            .and_then(|index| Plan::OFFERED.get(index).copied())
            .unwrap_or(answer);
        match Plan::parse(typed) {
            Ok(plan) => return Some(plan),
            Err(why) => prompt = format!("  {why}. Try again, or press Enter to skip: "),
        }
    }
    None
}

/// The one wording the plan question uses.
fn plan_question() -> String {
    use std::fmt::Write as _;
    let offered =
        Plan::OFFERED
            .iter()
            .enumerate()
            .fold(String::new(), |mut text, (index, plan)| {
                let _ = writeln!(text, "  {}) {plan}", index + 1);
                text
            });
    format!(
        concat!(
            "Which plan does this token belong to? claude reads it to describe the\n",
            "session and to pick its default model, and an injected token does not\n",
            "carry it.\n",
            "{}",
            "Type a number or a plan name, or press Enter to skip: "
        ),
        offered
    )
}

fn run_setup_token(
    context: &AppContext,
    account: &Identifier,
    program: &Path,
) -> Result<(), AppError> {
    let invocation = ChildInvocation::new(
        program.to_path_buf(),
        vec!["setup-token".into()],
        account_environment(context, account, None),
    );
    let outcome = if context.output_mode() == crate::commands::dispatch::OutputMode::Json {
        context
            .adapters()
            .process()
            .run_stdout_to_stderr(&invocation)
    } else {
        context.adapters().process().run_inherited(&invocation)
    };
    match outcome {
        Ok(ChildOutcome::Exited(0)) => Ok(()),
        Ok(other) => Err(child_failure("claude setup-token", other)),
        Err(error) => Err(error),
    }
}

/// Asks the child whether a candidate authenticates.
///
/// The captured output is never parsed beyond its exit status and never
/// interpolated into a diagnostic: the child's status document names an account
/// and an organization, and the wrapper's report carries neither.
pub(crate) fn probe(
    context: &AppContext,
    account: &Identifier,
    program: &Path,
    candidate: &Secret,
) -> Probe {
    let invocation = ChildInvocation::new(
        program.to_path_buf(),
        vec!["auth".into(), "status".into(), "--json".into()],
        account_environment(context, account, Some(candidate)),
    );
    match context.adapters().process().run_captured(&invocation) {
        Ok(captured) => match captured.outcome {
            ChildOutcome::Exited(0) => Probe {
                status: ProbeStatus::Ok,
                exit_code: Some(0),
            },
            ChildOutcome::Exited(code) => Probe {
                status: ProbeStatus::Failed,
                exit_code: Some(code),
            },
            ChildOutcome::Signaled(_) => Probe {
                status: ProbeStatus::Failed,
                exit_code: None,
            },
        },
        Err(_) => Probe {
            status: ProbeStatus::Unavailable,
            exit_code: None,
        },
    }
}

/// Turns a failed verification probe into the login's own failure.
pub(crate) fn verification_failure(probe: Probe) -> AppError {
    let mut diagnostic = Diagnostic::new(
        "the token was not accepted by the child",
        "claude auth status --json",
        match probe.status {
            ProbeStatus::Unavailable => "the child could not be run to verify the token".to_owned(),
            _ => "the child did not report a working credential for this token".to_owned(),
        },
        "check that the token is current, then run account login --token again",
    );
    diagnostic.child_exit = probe.exit_code;
    AppError::new(ErrorKind::Auth, diagnostic)
}

/// Commits a verified candidate as the account's stored token.
///
/// Under the credential lock, and in the one order the pair permits: the token
/// first, then the metadata whose rename is the commit. A crash between the two
/// therefore leaves a working token described by a stale fingerprint, which
/// `account status` detects and reports rather than a half-written file that
/// nothing could interpret.
///
/// The declared plan rides in that same metadata rename, which is what keeps it
/// from outliving the credential it describes: a rotation that does not
/// re-declare a plan clears it, and the account reports as undeclared rather
/// than carrying an answer given about a token that is gone.
///
/// Last comes the retirement of the child's own saved login, if the account
/// had one. It follows the rename for the reason everything else here does: the
/// rename is the commit, and an interruption before it must leave the previous
/// credential working.
pub(crate) fn rotate(
    context: &AppContext,
    account: &Identifier,
    candidate: &Secret,
    recorded_at: RecordedAt,
    plan: Option<Plan>,
) -> Result<ModeCommit, AppError> {
    let state = context.paths().state();
    let _lock = super::hold(context, account)?;
    let token_path = context.paths().account_oauth_token(account);
    guard::validate(state, &token_path, guard::Expected::PrivateFile)?;
    // No trailing newline. The file holds the value and nothing else, so one
    // fingerprint describes both the token and the file and no trimming policy
    // can drift between this writer and the reader below.
    atomic::write(&token_path, candidate.expose(), 0o600)?;
    let metadata = AuthModeMetadata {
        mode: AuthMode::Token,
        recorded_at,
        fingerprint: Some(Fingerprint::of(candidate)),
        plan,
    };
    super::write_metadata(context, account, &metadata)?;
    let retired = super::retire_superseded(context, account, AuthMode::Token)?;
    Ok(ModeCommit { metadata, retired })
}

/// Returns the ingest time to record for one request.
pub(crate) fn recorded_at(context: &AppContext, request: &TokenIngest) -> RecordedAt {
    request
        .minted_at
        .clone()
        .unwrap_or_else(|| RecordedAt::new_unchecked(rfc3339_utc(context.adapters().clock().now())))
}

/// Reads the account's stored token back.
///
/// Opened once, revalidated from the open handle, and read from that same
/// handle, so nothing swapped between the check and the read is what gets read.
/// Only two callers exist: the launch that injects the value, and the status
/// verb that fingerprints it. `account list` deliberately does neither — it
/// answers from local state alone, and consistency is a status fact.
pub(crate) fn read_stored(context: &AppContext, account: &Identifier) -> Result<Secret, AppError> {
    let path = context.paths().account_oauth_token(account);
    guard::validate(context.paths().state(), &path, guard::Expected::PrivateFile)?;
    let mut handle = SystemFileSystem::open_private_file(&path).map_err(|error| {
        AppError::new(
            ErrorKind::Auth,
            Diagnostic::new(
                "the stored token could not be read",
                path.display().to_string(),
                error.to_string(),
                "run claude-session-rs account login --token to store a working token",
            ),
        )
    })?;
    let mut bytes = Vec::new();
    handle.read_to_end(&mut bytes).map_err(|error| {
        AppError::new(
            ErrorKind::Io,
            Diagnostic::new(
                "the stored token could not be read",
                path.display().to_string(),
                error.to_string(),
                "check that the account directory is readable",
            ),
        )
    })?;
    Ok(Secret::from_stored(bytes))
}

/// Renders the estimated expiry of a token recorded at `minted`.
pub(crate) fn estimated_expiry(minted: &RecordedAt) -> Option<String> {
    let instant = crate::adapters::clock::parse_rfc3339_utc(minted.as_str())?;
    let lifetime = std::time::Duration::from_secs(ESTIMATED_LIFETIME_DAYS * 86_400);
    Some(rfc3339_utc(instant.checked_add(lifetime)?))
}

/// Returns the token's age in whole seconds, saturating at zero.
///
/// A mint time in the future is clock skew or a mistyped `--minted-at`, and
/// saturating reports it as brand new rather than underflowing into an age no
/// credential could have.
pub(crate) fn age_seconds(context: &AppContext, minted: &RecordedAt) -> Option<u64> {
    let minted = crate::adapters::clock::parse_rfc3339_utc(minted.as_str())?;
    Some(
        context
            .adapters()
            .clock()
            .now()
            .duration_since(minted)
            .map_or(0, |elapsed| elapsed.as_secs()),
    )
}

/// Builds the child environment for one token-mode subroutine.
///
/// The candidate, when present, goes in `CLAUDE_CODE_OAUTH_TOKEN`, which is the
/// child's own documented consumption path, and clears every ambient credential
/// that would outrank it — which is what makes the verification mean anything.
/// [`super::strip_ambient`] owns that reasoning; the mint run passes no
/// candidate, because it is asking the child to produce one rather than asking
/// about one.
fn account_environment(
    context: &AppContext,
    account: &Identifier,
    candidate: Option<&Secret>,
) -> Vec<(OsString, OsString)> {
    let mut environment = super::scoped_environment(context, account);
    if let Some(candidate) = candidate {
        super::strip_ambient(&mut environment);
        environment.push((
            "CLAUDE_CODE_OAUTH_TOKEN".into(),
            OsString::from(String::from_utf8_lossy(candidate.expose()).into_owned()),
        ));
    }
    environment
}

fn child_failure(command: &str, outcome: ChildOutcome) -> AppError {
    let mut diagnostic = Diagnostic::new(
        "the child token command failed",
        command.to_owned(),
        format!("{command} returned {outcome:?}"),
        "retry from an interactive terminal, or supply an existing token with --stdin",
    );
    diagnostic.child_exit = match outcome {
        ChildOutcome::Exited(code) => Some(code),
        ChildOutcome::Signaled(_) => None,
    };
    AppError::new(ErrorKind::Auth, diagnostic)
}
