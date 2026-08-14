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
//! `machineID`, and everything else in there, and the write below preserves all
//! of it ([ADR-0098]).
//!
//! The file lives in the terminal's own session directory rather than the
//! account's, so the seed happens when a launch creates that directory rather
//! than when a login creates the account. The same launch answers the child's
//! other first-run question, workspace trust, for the directory it was started
//! in and only while the user leaves that enabled
//! ([ADR-0105]).
//!
//! [ADR-0098]: ../../../docs/decisions/ADR-0098-seed-the-one-child-key-a-launch-cannot-reach.md
//! [ADR-0105]: ../../../docs/decisions/ADR-0105-seed-a-session-at-launch.md

use std::{
    io::Read as _,
    path::{Path, PathBuf},
};

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

/// The child's per-workspace map, and the two keys a trusted entry carries.
const PROJECTS: &str = "projects";
const TRUSTED: &str = "hasTrustDialogAccepted";
const ONBOARDED: &str = "hasCompletedProjectOnboarding";

/// Resolves the child configuration file of this terminal's session directory.
///
/// Deriving the terminal is a syscall, which is why this is a service rather
/// than a path method: `domain::paths` computes, and something has to ask the
/// operating system which terminal is asking.
fn session_config(context: &AppContext, account: &Identifier) -> Result<PathBuf, AppError> {
    let terminal = crate::services::session::terminal(context)?;
    Ok(context
        .paths()
        .session_native_config(account, terminal.namespace().id(), terminal.id()))
}

/// Names the file this terminal's launch would read, for a report to cite.
///
/// `None` where the terminal itself could not be derived, which is a defect
/// `session-terminal-derives` owns and reports on its own.
pub(crate) fn session_config_path(context: &AppContext, account: &Identifier) -> Option<PathBuf> {
    session_config(context, account).ok()
}

/// What a launch under one account would meet.
///
/// Two answers rather than three. A missing key used to be its own outcome,
/// repaired by logging in again; since [ADR-0105] moved the seed to launch,
/// `make_launchable` sets it before the exec, exactly as it does for the file
/// that is not there at all. Reporting it would name a state nothing can be in
/// by the time the child runs, and would print a remedy — run the login again —
/// that no longer touches this file. What remains is the state a launch cannot
/// repair: a file it cannot read or cannot understand.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Readiness {
    /// The child would go straight to its prompt.
    Ready,
    /// The answer could not be read, carrying why in the reader's words.
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
    let path = match session_config(context, account) {
        Ok(path) => path,
        Err(error) => return Readiness::Unreadable(error.diagnostic().why.clone()),
    };
    if let Err(error) =
        guard::validate(context.paths().state(), &path, guard::Expected::PrivateFile)
    {
        return Readiness::Unreadable(error.diagnostic().why.clone());
    }
    let bytes = match read_existing(&path) {
        // An absent file is a terminal that has not launched yet, and the
        // launch that creates the directory seeds the key on its way past
        // ([ADR-0105]). Reporting it as a defect would warn about a state
        // nothing can be in by the time the child runs.
        Ok(None) => return Readiness::Ready,
        Ok(Some(bytes)) => bytes,
        Err(error) => return Readiness::Unreadable(error),
    };
    match serde_json::from_slice::<serde_json::Value>(&bytes) {
        // The key itself is not consulted: a document the launch can read is a
        // document the launch can seed, whatever it currently says.
        Ok(serde_json::Value::Object(_)) => Readiness::Ready,
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
/// a login commits, and taking it here keeps this write from interleaving with a
/// concurrent login or with a removal that has already committed to destroying
/// the tree around it.
///
/// The lock reaches wrapper writers only. A `claude` already running under this
/// account holds nothing and writes this same file, so a key it sets between the
/// read and the rename below would be lost. `ADR-0084` leaves no wrapper alive
/// past the exec to hold anything on the child's behalf, so the window cannot be
/// closed (`docs/plan/open-questions.md`, Q-009) — it is narrowed instead. The
/// write happens only when this run would actually change a key, so a launch
/// into a session directory already carrying both answers reads and returns
/// without replacing a file a running child may be writing. Since [ADR-0105]
/// moved the seed to launch, that is the ordinary case: the file is replaced on
/// the launch that creates the directory and on the first launch from a new
/// workspace, and not otherwise.
pub(crate) fn make_launchable(
    context: &AppContext,
    account: &Identifier,
    trust: Option<&Path>,
) -> Result<(), AppError> {
    let path = session_config(context, account)?;
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
    let mut changed = document.insert(KEY.to_owned(), serde_json::Value::Bool(true))
        != Some(serde_json::Value::Bool(true));
    if let Some(directory) = trust {
        changed |= record_trust(&mut document, directory);
    }
    // Nothing to say means nothing to write. Replacing the file anyway is the
    // lost-update window above, opened once per launch for no gain.
    if !changed {
        return Ok(());
    }
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

/// Marks one workspace trusted, leaving every other entry as the child left it.
///
/// Reports whether the document changed, so a launch with nothing to say does
/// not replace a file a running child may be writing.
///
/// The two keys are written together because the child asks two questions about
/// a new workspace and answering one leaves the other prompt standing. A map
/// entry that is not an object is replaced rather than merged: the wrapper
/// cannot merge into something it cannot read, and this is per-workspace state
/// the child rebuilds, unlike the document around it. The same holds for the
/// map itself — a `projects` value that is not an object is replaced rather
/// than stepped around, because returning here would leave the launch reporting
/// success while the working directory stayed untrusted.
fn record_trust(
    document: &mut serde_json::Map<String, serde_json::Value>,
    directory: &Path,
) -> bool {
    let key = directory.display().to_string();
    let projects = document
        .entry(PROJECTS.to_owned())
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
    let mut changed = if projects.is_object() {
        false
    } else {
        *projects = serde_json::Value::Object(serde_json::Map::new());
        true
    };
    let Some(map) = projects.as_object_mut() else {
        return changed;
    };
    let entry = map
        .entry(key)
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
    if !entry.is_object() {
        *entry = serde_json::Value::Object(serde_json::Map::new());
        changed = true;
    }
    if let Some(workspace) = entry.as_object_mut() {
        changed |= workspace.insert(TRUSTED.to_owned(), serde_json::Value::Bool(true))
            != Some(serde_json::Value::Bool(true));
        changed |= workspace.insert(ONBOARDED.to_owned(), serde_json::Value::Bool(true))
            != Some(serde_json::Value::Bool(true));
    }
    changed
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

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    fn document(text: &str) -> serde_json::Map<String, serde_json::Value> {
        match serde_json::from_str(text).expect("the fixture parses") {
            serde_json::Value::Object(map) => map,
            other => panic!("the fixture is not an object: {other}"),
        }
    }

    fn trusted(document: &serde_json::Map<String, serde_json::Value>, key: &str) -> bool {
        document
            .get(PROJECTS)
            .and_then(|projects| projects.get(key))
            .and_then(|entry| entry.get(TRUSTED))
            == Some(&serde_json::Value::Bool(true))
    }

    /// The launch directory is recorded, and every other entry is left as the
    /// child wrote it.
    #[test]
    fn a_new_workspace_is_recorded_beside_the_ones_already_there() {
        let mut subject = document(r#"{"projects":{"/other":{"hasTrustDialogAccepted":true}}}"#);
        assert!(record_trust(&mut subject, Path::new("/work")));
        assert!(trusted(&subject, "/work"));
        assert!(trusted(&subject, "/other"));
    }

    /// Nothing to say means nothing to write, which is what keeps an ordinary
    /// launch from replacing a file a running child may be writing.
    #[test]
    fn a_workspace_already_trusted_reports_no_change() {
        let mut subject = document("{}");
        assert!(record_trust(&mut subject, Path::new("/work")));
        assert!(!record_trust(&mut subject, Path::new("/work")));
    }

    /// A `projects` value the wrapper cannot merge into is replaced rather than
    /// stepped around: returning would leave the launch reporting success with
    /// the working directory still untrusted.
    #[test]
    fn a_projects_value_that_is_not_a_map_is_replaced_rather_than_skipped() {
        for text in [r#"{"projects":[]}"#, r#"{"projects":"none"}"#] {
            let mut subject = document(text);
            assert!(record_trust(&mut subject, Path::new("/work")), "{text}");
            assert!(trusted(&subject, "/work"), "{text}");
        }
    }

    /// The same holds one level down, for per-workspace state the child rebuilds.
    #[test]
    fn a_workspace_entry_that_is_not_a_map_is_replaced() {
        let mut subject = document(r#"{"projects":{"/work":7}}"#);
        assert!(record_trust(&mut subject, Path::new("/work")));
        assert!(trusted(&subject, "/work"));
    }

    /// The child asks two questions about a new workspace, and answering one
    /// leaves the other prompt standing.
    #[test]
    fn both_first_run_answers_are_written_together() {
        let mut subject = document("{}");
        assert!(record_trust(&mut subject, Path::new("/work")));
        let entry = subject
            .get(PROJECTS)
            .and_then(|projects| projects.get("/work"))
            .expect("the workspace entry exists");
        assert_eq!(entry.get(TRUSTED), Some(&serde_json::Value::Bool(true)));
        assert_eq!(entry.get(ONBOARDED), Some(&serde_json::Value::Bool(true)));
    }
}
