//! Resolving this run's account and profile selection into paths.
//!
//! This module is for turning a selection into paths and preparing them on
//! first use. It is not for authentication, and it puts nothing on the child's
//! argument vector.

use std::path::PathBuf;

use crate::{
    adapters::terminal::Terminal as _,
    context::AppContext,
    domain::{checks::Check, identifier::Identifier, terminal::Terminal},
    error::{AppError, Diagnostic},
    services::storage::guard,
};

/// Names the terminal this run's child state directory belongs to, or refuses.
///
/// The refusal is the ladder's last rung and is deliberate: with nothing to
/// name, the only alternatives are sharing another terminal's state, which is
/// what the separation exists to prevent, or inventing a directory the user
/// can never find again ([ADR-0102]).
///
/// [ADR-0102]: ../../docs/decisions/ADR-0102-key-child-state-by-terminal.md
pub(crate) fn terminal(context: &AppContext) -> Result<Terminal, AppError> {
    let derived = context
        .adapters()
        .terminal()
        .identity()
        .map_err(|error| refuse(&error.to_string()))?;
    derived.ok_or_else(|| refuse("neither a controlling terminal nor a session leader named one"))
}

fn refuse(why: &str) -> AppError {
    AppError::new(
        Check::SessionTerminalDerives.kind(),
        Diagnostic::new(
            "this run's terminal could not be named",
            "the controlling terminal and the session leader",
            why.to_owned(),
            Check::SessionTerminalDerives
                .remediation()
                .unwrap_or_default(),
        ),
    )
}

/// Prepares one terminal's child state directory and the tree it shares back.
///
/// Four steps, in this order: the terminal's namespace directory, the session
/// directory inside it, the account's projects tree, and the declared link
/// between them. The tree is created before the link so the link is never
/// dangling, which is what lets the guard verify it on the next run
/// ([ADR-0103]).
///
/// Idempotent. A second run of the same terminal validates what the first one
/// built and creates nothing.
///
/// [ADR-0103]: ../../docs/decisions/ADR-0103-permit-a-declared-link.md
/// [ADR-0107]: ../../docs/decisions/ADR-0107-scope-a-terminal-to-its-namespace.md
pub(crate) fn materialise(
    context: &AppContext,
    account: &Identifier,
    terminal: &Terminal,
) -> Result<PathBuf, AppError> {
    let paths = context.paths();
    let state = paths.state();
    let namespace = terminal.namespace().id();
    // Its own component, validated in its own right: a terminal name is unique
    // only inside the namespace that issued it ([ADR-0107]).
    guard::ensure_directory(state, &paths.account_namespace(account, namespace))?;
    let directory = paths.account_session(account, namespace, terminal.id());
    guard::ensure_directory(state, &directory)?;
    let projects = paths.account_projects(account);
    guard::ensure_directory(state, &projects)?;
    guard::ensure_link(
        state,
        &paths.session_projects_link(account, namespace, terminal.id()),
        &projects,
    )?;
    tracing::info!(
        op = "materialise_session",
        status = "ok",
        terminal = terminal.id().as_str(),
        namespace = namespace.as_str(),
        path = %directory.display(),
        "this run will use the session directory at {}",
        directory.display()
    );
    Ok(directory)
}

/// The paths one run's selection resolves to.
///
/// Pure: building this reads configuration and computes paths, and touches no
/// filesystem. That is what makes it safe to hold on the immutable context,
/// where a validation result would not be.
#[derive(Clone, Debug)]
pub(crate) struct SessionPaths {
    account: Option<Account>,
    profile: Option<Identifier>,
}

/// One account's two directories.
#[derive(Clone, Debug)]
pub(crate) struct Account {
    /// The account identifier.
    pub(crate) id: Identifier,
    /// The account directory.
    pub(crate) directory: PathBuf,
    /// The child-owned native configuration directory.
    pub(crate) config: PathBuf,
}

impl SessionPaths {
    /// Resolves the selection without touching the filesystem.
    pub(crate) fn resolve(context: &AppContext) -> Self {
        let paths = context.paths();
        Self {
            account: context.account_selection().account().map(|id| Account {
                id: id.clone(),
                directory: paths.account(id),
                config: paths.account_config(id),
            }),
            profile: context.config().profile().cloned(),
        }
    }

    /// Returns the selected account, if any.
    pub(crate) const fn account(&self) -> Option<&Account> {
        self.account.as_ref()
    }
    /// Returns the selected profile, if any.
    pub(crate) const fn profile(&self) -> Option<&Identifier> {
        self.profile.as_ref()
    }
}
