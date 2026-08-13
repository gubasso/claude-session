//! Token-mode ingest, verification, commit, and read-back.
//!
//! The order here is the ingest sequence in `docs/reference/accounts.md`, and
//! two of its properties are structural rather than incidental. The candidate
//! reaches the child through the child's environment and never through argv, so
//! it cannot appear in a process listing. And verification happens before the
//! credential lock is taken, so the lock is never held across a child spawn.

use std::{
    ffi::OsString,
    io::Read as _,
    path::{Path, PathBuf},
};

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
            AmbientCredential, AuthMode, AuthModeMetadata, Plan, Probe, ProbeStatus, RecordedAt,
            TokenIngest, TokenSource,
        },
        child::{ChildInvocation, ChildOutcome},
        identifier::Identifier,
        secret::{Fingerprint, Secret, SecretError},
    },
    error::{AppError, Diagnostic, ErrorKind},
    services::storage::{atomic, guard},
};

/// The estimated lifetime of a long-lived subscription token.
///
/// An estimate rather than a reading: the wrapper does not parse the token, and
/// the child warns about an approaching saved-login expiry but gives no
/// equivalent notice for an injected one, which simply stops working. Derived
/// from the mint time, it is the best a wrapper that refuses to inspect a
/// credential can offer, and every surface labels it as an estimate.
pub(crate) const ESTIMATED_LIFETIME_DAYS: u64 = 365;

/// The escape a confirming or prompting token login names when it refuses.
const STDIN_ESCAPE: &str = "run account login --token --stdin with the token on standard input";

/// Obtains one token candidate from a terminal or from standard input.
pub(crate) fn ingest(
    context: &AppContext,
    account: &Identifier,
    program: &Path,
    request: &TokenIngest,
) -> Result<Secret, AppError> {
    match request.source {
        TokenSource::Stdin => read_stdin(),
        TokenSource::Terminal => {
            require_terminal(context)?;
            // Inherited streams, so the token the child prints goes to the
            // user's terminal and never through a pipe this process reads. The
            // wrapper then asks for it back, which is what keeps the value out
            // of any buffer the wrapper owns until the user chooses to paste
            // it.
            run_setup_token(context, account, program)?;
            read_terminal(context)
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

fn require_terminal(context: &AppContext) -> Result<(), AppError> {
    let unavailable = |why: String| {
        AppError::new(
            ErrorKind::Unavailable,
            Diagnostic::new(
                "token entry needs a controlling terminal",
                "/dev/tty",
                why,
                STDIN_ESCAPE,
            ),
        )
    };
    match context.adapters().terminal().available() {
        Ok(true) => Ok(()),
        Ok(false) => Err(unavailable(
            "no controlling terminal is available".to_owned(),
        )),
        Err(error) => Err(unavailable(error.to_string())),
    }
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

fn read_terminal(context: &AppContext) -> Result<Secret, AppError> {
    context
        .adapters()
        .terminal()
        .read_secret("Paste the token, then press Enter (it is not echoed): ")
        .map_err(|error| {
            AppError::new(
                ErrorKind::Unavailable,
                Diagnostic::new(
                    "the token could not be read from the terminal",
                    "/dev/tty",
                    error.to_string(),
                    STDIN_ESCAPE,
                ),
            )
        })?
        .map_err(|reason| refused(reason, "the pasted token"))
}

/// Reads standard input to end of file and requires exactly one line.
///
/// To end of file rather than one line: a reader that took the first line and
/// discarded the rest would silently accept a two-line paste and store half of
/// what the user meant. Bounded by [`Secret::INGEST_BOUND`] rather than by the
/// stream's own end, so the refusal costs one byte past the limit instead of
/// however much a pipe chooses to send.
fn read_stdin() -> Result<Secret, AppError> {
    let bytes = read_bounded(std::io::stdin().lock()).map_err(|error| {
        AppError::new(
            ErrorKind::NoInput,
            Diagnostic::new(
                "the token could not be read from standard input",
                "standard input",
                error.to_string(),
                "supply the token on standard input, as one line",
            ),
        )
    })?;
    Secret::parse_line(&bytes).map_err(|reason| refused(reason, "standard input"))
}

/// Reads at most [`Secret::INGEST_BOUND`] bytes from one ingest stream.
///
/// Taking one byte past the limit is what keeps the refusal and the bound in
/// the same place: the caller's `parse_line` still decides, and it decides from
/// a buffer that can never exceed a credential's accepted length by more than
/// the single byte that proves it was exceeded.
fn read_bounded(reader: impl std::io::Read) -> std::io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader.take(Secret::INGEST_BOUND).read_to_end(&mut bytes)?;
    Ok(bytes)
}

fn refused(reason: SecretError, where_: &str) -> AppError {
    AppError::new(
        ErrorKind::Usage,
        Diagnostic::new(
            "the token was not accepted",
            where_.to_owned(),
            reason.reason().to_owned(),
            "supply exactly one line holding the token and nothing else",
        ),
    )
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
pub(crate) fn rotate(
    context: &AppContext,
    account: &Identifier,
    candidate: &Secret,
    recorded_at: RecordedAt,
    plan: Option<Plan>,
) -> Result<AuthModeMetadata, AppError> {
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
    Ok(metadata)
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

/// Builds the child environment for one account-scoped subroutine.
///
/// The candidate, when present, goes in `CLAUDE_CODE_OAUTH_TOKEN`, which is the
/// child's own documented consumption path.
///
/// A candidate also clears every ambient credential the wrapper can see, and
/// that is what makes the verification mean anything. The child's precedence
/// ladder puts a bearer token, an API key, and a cloud provider selector above
/// an injected token — the same ladder the wrapper warns about before a launch
/// — so a probe that inherited one would report the ambient credential's
/// health and accept any string as a working token.
///
/// This is not the launch, where those mechanisms are never stripped because
/// they are the user's choice about their own session. It is a question the
/// wrapper asks about one specific credential, and the answer has to be about
/// that credential.
///
/// The one mechanism this cannot clear is `apiKeyHelper`, which lives in the
/// child's settings rather than the environment. Clearing it would mean writing
/// a settings document, and the wrapper does not author one to ask a question.
fn account_environment(
    context: &AppContext,
    account: &Identifier,
    candidate: Option<&Secret>,
) -> Vec<(OsString, OsString)> {
    let mut environment = crate::services::child::subroutine_environment(context);
    environment.retain(|(key, _)| key != "CLAUDE_CONFIG_DIR" && key != "CLAUDE_CODE_OAUTH_TOKEN");
    environment.push((
        "CLAUDE_CONFIG_DIR".into(),
        config_directory(context, account).into_os_string(),
    ));
    if let Some(candidate) = candidate {
        environment.retain(|(key, _)| {
            !AmbientCredential::ALL
                .iter()
                .any(|credential| key == credential.spelling())
        });
        environment.push((
            "CLAUDE_CODE_OAUTH_TOKEN".into(),
            OsString::from(String::from_utf8_lossy(candidate.expose()).into_owned()),
        ));
    }
    environment
}

fn config_directory(context: &AppContext, account: &Identifier) -> PathBuf {
    context.paths().account_config(account)
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

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    /// The bound has to live on the read rather than on the parse, or an ingest
    /// source that never reaches end of file would be held in memory in full
    /// before the length check could refuse it. `io::repeat` is exactly that
    /// source: this test does not terminate at all if the read is unbounded.
    /// The bound, as a length, for the two tests that build inputs from it.
    fn bound() -> usize {
        usize::try_from(Secret::INGEST_BOUND).expect("the ingest bound fits a usize")
    }

    #[test]
    fn an_endless_ingest_stream_is_refused_without_being_consumed() {
        let bytes = read_bounded(std::io::repeat(b'a')).expect("a bounded read succeeds");
        assert_eq!(bytes.len(), bound());
        assert_eq!(
            Secret::parse_line(&bytes).err(),
            Some(SecretError::TooLong),
            "one byte past the limit is what proves the limit was exceeded"
        );
    }

    /// The bound is one byte past the accepted length, so a credential exactly
    /// at the limit still survives the same reader.
    #[test]
    fn a_credential_at_the_limit_survives_the_bounded_read() {
        let source = vec![b'a'; bound() - 1];
        let bytes = read_bounded(source.as_slice()).expect("a bounded read succeeds");
        assert_eq!(bytes, source);
        assert!(Secret::parse_line(&bytes).is_ok());
    }
}
