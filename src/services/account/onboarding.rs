//! Whether an account can reach the child's prompt, and the one key that decides it.
//!
//! The child runs its first-run onboarding when `hasCompletedOnboarding` is not
//! `true` in the `.claude.json` of the configuration directory it was pointed
//! at, and it decides that without consulting authentication. Every account owns
//! a fresh directory, so an account the wrapper just authenticated would meet
//! that wizard — in token mode, a browser sign-in it does not need and cannot
//! complete into its stored mode.
//!
//! This module is for answering that one question and for writing that one
//! answer. It is not for the rest of the file: the child owns `userID`,
//! `machineID`, its per-workspace trust records, and everything else in there,
//! and the write below preserves all of it
//! ([ADR-0098](../../../docs/decisions/ADR-0098-seed-the-one-child-key-a-launch-cannot-reach.md)).

use std::io::Read as _;

use crate::{
    adapters::filesystem::SystemFileSystem,
    context::AppContext,
    domain::identifier::Identifier,
    error::{AppError, Diagnostic, ErrorKind},
    services::storage::{atomic, guard},
};

/// The child's own key. Carried against the launch obligation (ADR-0089), and
/// registered in `docs/reference/child-facts.yaml`.
const KEY: &str = "hasCompletedOnboarding";

/// What a launch under one account would meet.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Readiness {
    /// The child would go straight to its prompt.
    Ready,
    /// The child would run its first-run onboarding.
    WouldOnboard,
    /// The answer could not be read, carrying why in the reader's words.
    ///
    /// Separate from [`Self::WouldOnboard`] because the two need different next
    /// actions: one is repaired by logging in again, and the other by dealing
    /// with whatever is at the path.
    Unreadable(String),
}

impl Readiness {
    /// Reports whether a launch would reach the prompt.
    pub(crate) const fn ready(&self) -> bool {
        matches!(*self, Self::Ready)
    }
}

/// Answers the question without changing the file's content.
///
/// Not without changing the file at all: this path is guarded like every other
/// wrapper-managed path, and `storage-secret-modes` is "correct, then proceed",
/// so a `.claude.json` the child left at a wider mode is restricted to `0600`
/// here. That is the same treatment the child's own `.credentials.json` already
/// gets from `credentials-usable`, and it corrects a permission rather than a
/// key: the content stays the child's.
///
/// Never an error. Three callers ask it — a launch about to exec, `doctor`, and
/// `account status` — and none of them may fail over the answer: a launch that
/// would work must not be refused because a report could not be written, and a
/// diagnostic verb that cannot run is the one that was supposed to explain the
/// defect.
pub(crate) fn readiness(context: &AppContext, account: &Identifier) -> Readiness {
    let path = context.paths().account_native_config(account);
    if let Err(error) =
        guard::validate(context.paths().state(), &path, guard::Expected::PrivateFile)
    {
        return Readiness::Unreadable(error.diagnostic().why.clone());
    }
    let bytes = match read_existing(&path) {
        // An absent file is the new account's ordinary state, not a fault.
        Ok(None) => return Readiness::WouldOnboard,
        Ok(Some(bytes)) => bytes,
        Err(error) => return Readiness::Unreadable(error),
    };
    match serde_json::from_slice::<serde_json::Value>(&bytes) {
        Ok(serde_json::Value::Object(document)) => {
            if document.get(KEY) == Some(&serde_json::Value::Bool(true)) {
                Readiness::Ready
            } else {
                Readiness::WouldOnboard
            }
        }
        Ok(_) => Readiness::Unreadable(not_an_object()),
        Err(error) => Readiness::Unreadable(error.to_string()),
    }
}

/// Records that the child's first-run onboarding is done, preserving the rest.
///
/// Read-modify-write rather than a fresh document: the child's trust records for
/// every workspace live in this file, and replacing it would silently withdraw
/// them. A file that is not an object is refused rather than replaced, because
/// the wrapper cannot tell an unreadable child file from one it does not
/// understand yet, and destroying it would be the same act either way.
///
/// The caller holds the account's credential lock. That scope guards the files
/// a login commits, and this write is part of the same login: taking it here
/// keeps a login from interleaving with a concurrent one or with a removal that
/// has already committed to destroying the tree around it.
///
/// The lock reaches wrapper writers only. A `claude` already running under this
/// account holds nothing and writes this same file, so a key it sets between the
/// read above and the rename below is lost. That window is why the write is a
/// login's and never a launch's, and it is open rather than closed: `ADR-0084`
/// leaves no wrapper alive past the exec to hold anything on the child's behalf
/// (`docs/plan/open-questions.md`, Q-009).
pub(crate) fn make_launchable(context: &AppContext, account: &Identifier) -> Result<(), AppError> {
    let path = context.paths().account_native_config(account);
    guard::validate(context.paths().state(), &path, guard::Expected::PrivateFile)?;
    let mut document = match read_existing(&path) {
        Ok(None) => serde_json::Map::new(),
        Ok(Some(bytes)) => match serde_json::from_slice::<serde_json::Value>(&bytes) {
            Ok(serde_json::Value::Object(existing)) => existing,
            Ok(_) => return Err(malformed(&path, not_an_object())),
            Err(error) => return Err(malformed(&path, error.to_string())),
        },
        Err(why) => return Err(unreadable(&path, why)),
    };
    document.insert(KEY.to_owned(), serde_json::Value::Bool(true));
    let mut bytes = serde_json::to_vec(&document).map_err(|error| {
        AppError::new(
            ErrorKind::Internal,
            Diagnostic::new(
                "the account's child configuration could not be encoded",
                account.as_str(),
                error.to_string(),
                "report this invariant",
            ),
        )
    })?;
    bytes.push(b'\n');
    atomic::write(&path, &bytes, 0o600)
}

/// Reads a file that may not exist, separating absence from a read failure.
fn read_existing(path: &std::path::Path) -> Result<Option<Vec<u8>>, String> {
    let mut handle = match SystemFileSystem::open_private_file(path) {
        Ok(handle) => handle,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.to_string()),
    };
    let mut bytes = Vec::new();
    handle
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    Ok(Some(bytes))
}

fn not_an_object() -> String {
    "the child's configuration file does not hold a JSON object".to_owned()
}

fn malformed(path: &std::path::Path, why: String) -> AppError {
    AppError::new(
        ErrorKind::DataFormat,
        Diagnostic::new(
            "the account's child configuration could not be understood",
            path.display().to_string(),
            why,
            "Move the file above aside, then run the login again.",
        ),
    )
}

fn unreadable(path: &std::path::Path, why: String) -> AppError {
    AppError::new(
        ErrorKind::Io,
        Diagnostic::new(
            "the account's child configuration could not be read",
            path.display().to_string(),
            why,
            "Check the file above, then run the login again.",
        ),
    )
}
