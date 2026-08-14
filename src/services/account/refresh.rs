//! Handing a refresh token to the child, which exchanges it for a saved login.
//!
//! The whole module is a delegation. The wrapper reads one secret, puts it in
//! the child's environment beside the scopes it was issued with, and runs the
//! child's own `auth login`; the exchange, the credential file, the profile,
//! the organization roles, and the first-run key are all the child's work and
//! none of them is inspected here. That is what keeps ADR-0026's
//! refusal to speak an OAuth endpoint intact while still reaching a credential
//! only that endpoint can issue
//! ([ADR-0100](../../../docs/decisions/ADR-0100-bootstrap-a-saved-login-from-a-refresh-token.md)).
//!
//! Nothing here is stored. The refresh token lives in this process and in the
//! child's environment, and is gone when both exit.

use std::{ffi::OsString, path::Path};

use crate::{
    adapters::process::ProcessRunner,
    context::AppContext,
    domain::{
        account::{RefreshIngest, Scopes, TokenSource},
        child::{ChildInvocation, ChildOutcome},
        identifier::Identifier,
        secret::Secret,
    },
    error::{AppError, Diagnostic, ErrorKind},
};

use super::ingest as source;

/// The credential this module reads, for the shared ingest's wording.
const SUBJECT: source::Subject = source::Subject::RefreshToken;

/// Obtains one refresh token from a terminal or from standard input.
///
/// No child runs first, unlike the token ingest: there is nothing for the
/// wrapper to ask the child to mint, because a refresh token is issued only by
/// a login that already happened somewhere else.
pub(crate) fn ingest(context: &AppContext, request: &RefreshIngest) -> Result<Secret, AppError> {
    match request.source {
        TokenSource::Stdin => source::read_stdin(SUBJECT),
        TokenSource::Terminal => {
            source::require_terminal(context, SUBJECT)?;
            source::read_terminal(
                context,
                SUBJECT,
                "Paste the refresh token, then press Enter (it is not echoed): ",
            )
        }
    }
}

/// Runs the child's own login against a supplied refresh token.
///
/// Argv is exactly `auth login`, with no subscription selector: the child's
/// refresh branch never reaches the flag that would pick one, so naming it
/// would carry a second child-owned spelling that changes nothing (ADR-0089).
///
/// The secret reaches the child through its environment and never through an
/// argument, so it cannot appear in a process listing — the same property the
/// token ingest has.
///
/// Streams follow the native login's: the child's own `Login successful.` or
/// `Login failed: …` is the most useful thing anyone gets out of this, and it
/// is diverted to standard error under `--json` so the report stays the only
/// thing on standard output.
pub(crate) fn bootstrap(
    context: &AppContext,
    account: &Identifier,
    program: &Path,
    secret: &Secret,
    scopes: &Scopes,
) -> Result<(), AppError> {
    let invocation = ChildInvocation::new(
        program.to_path_buf(),
        vec!["auth".into(), "login".into()],
        environment(context, account, secret, scopes),
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
        Ok(other) => Err(failure(other)),
        Err(error) => Err(error),
    }
}

/// Builds the child environment for the exchange.
///
/// Ambient credentials go for the reason the verification probe drops them
/// ([`super::strip_ambient`]): this is the wrapper asking the child about one
/// supplied credential, and an inherited one would answer instead.
fn environment(
    context: &AppContext,
    account: &Identifier,
    secret: &Secret,
    scopes: &Scopes,
) -> Vec<(OsString, OsString)> {
    use std::os::unix::ffi::OsStringExt as _;

    let mut environment = super::scoped_environment(context, account);
    super::strip_ambient(&mut environment);
    environment.retain(|(key, _)| {
        key != "CLAUDE_CODE_OAUTH_REFRESH_TOKEN" && key != "CLAUDE_CODE_OAUTH_SCOPES"
    });
    // From the bytes rather than through a string: the ingest accepts every
    // non-control byte precisely so a change in the provider's token format
    // cannot make it refuse a working credential, and a lossy UTF-8 conversion
    // here would undo that by substituting U+FFFD into a secret the wrapper
    // already accepted. An environment value is an OS string to the boundary
    // ([coding conventions](../../../docs/reference/coding-conventions.md#types)).
    environment.push((
        "CLAUDE_CODE_OAUTH_REFRESH_TOKEN".into(),
        OsString::from_vec(secret.expose().to_vec()),
    ));
    environment.push((
        "CLAUDE_CODE_OAUTH_SCOPES".into(),
        OsString::from(scopes.as_str()),
    ));
    environment
}

/// Turns a failed exchange into the login's own failure.
///
/// The child's own message already went to the user's terminal, so this names
/// the command and its status and does not try to restate a reason it never
/// parsed. The scopes are the one thing worth pointing at: a grant that does
/// not hold what was asked for is the failure this path invites.
fn failure(outcome: ChildOutcome) -> AppError {
    let mut diagnostic = Diagnostic::new(
        "the refresh token was not exchanged",
        "claude auth login",
        format!("claude auth login returned {outcome:?}"),
        concat!(
            "check that the refresh token is current and that --scopes matches what it ",
            "was issued with, then run account login --refresh-token again"
        ),
    );
    diagnostic.child_exit = match outcome {
        ChildOutcome::Exited(code) => Some(code),
        ChildOutcome::Signaled(_) => None,
    };
    AppError::new(ErrorKind::Auth, diagnostic)
}
