//! Resolving this run's account and profile selection into paths.
//!
//! This module is for turning a selection into paths and preparing them on
//! first use. It is not for authentication, and it puts nothing on the child's
//! argument vector.

use std::path::{Path, PathBuf};

use crate::{
    context::AppContext,
    domain::{agent::Agent, checks::Check, identifier::Identifier, witness::Marker},
    error::{AppError, Diagnostic},
    services::storage::{atomic, guard},
};

pub(crate) mod gc;

/// Names the agent this run is about to become, or refuses.
///
/// The refusal is deliberate: with nothing to name, the only alternatives are
/// sharing another agent's state, which is what the separation exists to
/// prevent, or inventing a directory the user can never find again. It needs a
/// `/proc` that answers, which is the same precondition every other identity
/// read here has ([ADR-0113]).
///
/// [ADR-0113]: ../../docs/decisions/ADR-0113-key-a-session-to-its-running-agent.md
pub(crate) fn agent(context: &AppContext) -> Result<Agent, AppError> {
    let _ = context;
    crate::adapters::host::agent()
        .ok_or_else(|| refuse("this process's namespace and start time could not be read"))
}

fn refuse(why: &str) -> AppError {
    AppError::new(
        Check::SessionIdentityDerives.kind(),
        Diagnostic::new(
            "this run's agent could not be named",
            "this process's identity under /proc",
            why.to_owned(),
            Check::SessionIdentityDerives
                .remediation()
                .unwrap_or_default(),
        ),
    )
}

/// Names the session directory this run is inside, when it is inside one.
///
/// A command is never an agent, so the session directory a `doctor` run should
/// look at is not one it would create — that directory does not exist until a
/// launch makes it — but the agent this command is running under, if any. The
/// ancestry says which, and the directory's own name is what confirms it: the
/// name spells the process and its start time, so an ancestor that names an
/// existing directory is that directory's agent ([ADR-0113]).
///
/// [ADR-0113]: ../../docs/decisions/ADR-0113-key-a-session-to-its-running-agent.md
pub(crate) fn current(context: &AppContext, account: &Identifier) -> Option<PathBuf> {
    let namespace = crate::adapters::host::namespace(crate::domain::namespace::Kind::Pid)?;
    crate::adapters::host::lineage()
        .into_iter()
        .filter_map(|(pid, started)| Agent::new(namespace.clone(), pid, started))
        .find(|agent| witnesses(context, account, agent))
        .map(|agent| {
            context
                .paths()
                .account_session(account, agent.namespace().id(), agent.id())
        })
}

/// Reports whether a session directory stands for this exact agent.
///
/// A name on its own is a guess: a directory can be moved into place, and the
/// path is reached by following links. The record beside it is the evidence, so
/// an ancestor names this run's session only when the directory is there and a
/// record filed under that namespace names that same process ([ADR-0113]).
///
/// [ADR-0113]: ../../docs/decisions/ADR-0113-key-a-session-to-its-running-agent.md
fn witnesses(context: &AppContext, account: &Identifier, agent: &Agent) -> bool {
    let paths = context.paths();
    let namespace = agent.namespace().id();
    if !paths
        .account_session(account, namespace, agent.id())
        .is_dir()
    {
        return false;
    }
    std::fs::read(paths.session_witness(account, namespace, agent.id()))
        .ok()
        .and_then(|bytes| Marker::from_bytes(&bytes))
        .is_some_and(|marker| {
            marker.pid() == agent.pid()
                && marker.started() == agent.started()
                && marker.namespace() == namespace
        })
}

/// Prepares one agent's child state directory and the tree it shares back.
///
/// Five steps, in this order: the dead sessions this account left behind, the
/// agent's namespace directory, the session directory inside it, the account's
/// projects tree, and the declared link between them. The tree is created
/// before the link so the link is never dangling, which is what lets the guard
/// verify it on the next run ([ADR-0103]).
///
/// Not idempotent, and cannot be: a session is one agent run, so a second
/// launch is a second agent and gets a directory of its own ([ADR-0113]).
///
/// [ADR-0103]: ../../docs/decisions/ADR-0103-permit-a-declared-link.md
/// [ADR-0113]: ../../docs/decisions/ADR-0113-key-a-session-to-its-running-agent.md
pub(crate) fn materialise(
    context: &AppContext,
    account: &Identifier,
    agent: &Agent,
) -> Result<PathBuf, AppError> {
    let paths = context.paths();
    let state = paths.state();
    let namespace = agent.namespace().id();
    // Additive, and first: one directory per launch would otherwise grow
    // without bound between explicit collections. A sweep that fails costs a
    // tidy tree, never the launch ([ADR-0113]).
    match gc::sweep(context, account) {
        Ok(0) => {}
        Ok(removed) => tracing::info!(
            op = "sweep_sessions",
            status = "ok",
            removed,
            "collected {removed} session directories whose agent had exited"
        ),
        Err(error) => tracing::warn!(
            op = "sweep_sessions",
            status = "degraded",
            "this account's exited sessions were not collected: {}",
            error.diagnostic().why
        ),
    }
    // Its own component, validated in its own right: a process identifier is
    // unique only inside the namespace that issued it.
    guard::ensure_directory(state, &paths.account_namespace(account, namespace))?;
    let directory = paths.account_session(account, namespace, agent.id());
    guard::ensure_directory(state, &directory)?;
    let projects = paths.account_projects(account);
    guard::ensure_directory(state, &projects)?;
    guard::ensure_link(
        state,
        &paths.session_projects_link(account, namespace, agent.id()),
        &projects,
    )?;
    // Additive, like the registry share below: a launch that cannot record
    // its witness leaves the session unaccounted for, and an unaccounted
    // session is collectable, so the cost is a directory a later collection
    // takes rather than an exec this run refuses ([ADR-0110]).
    //
    // [ADR-0110]: ../../docs/decisions/ADR-0110-record-the-terminal-witness-at-launch.md
    if let Err(error) = record_witness(context, account, agent) {
        tracing::warn!(
            op = "record_witness",
            status = "degraded",
            "this session's agent witness was not recorded: {}",
            error.diagnostic().why
        );
    }
    // Additive, so a failure degrades to an unshared launch instead of
    // refusing the exec ([ADR-0108]): the child's peer discovery is a
    // convenience, and a damaged shared registry must not stop every
    // agent of every account from launching.
    //
    // [ADR-0108]: ../../docs/decisions/ADR-0108-share-the-child-peer-registry-across-sessions.md
    if let Err(error) = share_registry(context, account, agent) {
        tracing::warn!(
            op = "share_peer_registry",
            status = "degraded",
            "this run's session cannot reach the shared peer registry: {}",
            error.diagnostic().why
        );
    }
    tracing::info!(
        op = "materialise_session",
        status = "ok",
        session = agent.id().as_str(),
        namespace = namespace.as_str(),
        path = %directory.display(),
        "this run will use the session directory at {}",
        directory.display()
    );
    Ok(directory)
}

/// Records the witness of this agent beside its session directory.
///
/// One directory belongs to one agent run, so there is nothing to compare
/// against: the record is written once, for a directory that did not exist a
/// moment ago ([ADR-0110]).
///
/// [ADR-0110]: ../../docs/decisions/ADR-0110-record-the-terminal-witness-at-launch.md
fn record_witness(
    context: &AppContext,
    account: &Identifier,
    agent: &Agent,
) -> Result<(), AppError> {
    let degraded = |why: &str| {
        AppError::new(
            crate::error::ErrorKind::Io,
            Diagnostic::new(
                "the agent witness could not be recorded",
                "the session witness record",
                why.to_owned(),
                "the session will be judged unknown and never collected",
            ),
        )
    };
    let marker = Marker::from_agent(agent, crate::adapters::host::boot_id())
        .ok_or_else(|| degraded("this kernel's boot identifier could not be read"))?;
    let bytes = marker
        .to_bytes()
        .ok_or_else(|| degraded("the record did not serialize"))?;
    let paths = context.paths();
    let path = paths.session_witness(account, agent.namespace().id(), agent.id());
    // The guard runs before the unchanged-bytes fast path, not after: the
    // comparison must never read through a symbolic link or a foreign owner,
    // and a widened mode is corrected even when the bytes have not changed.
    guard::validate(paths.state(), &path, guard::Expected::PrivateFile)?;
    if std::fs::read(&path).is_ok_and(|existing| existing == bytes) {
        return Ok(());
    }
    atomic::write(&path, &bytes, 0o600)
}

/// Links one session directory's `sessions` name to the shared peer registry.
///
/// Three steps: the registry directory for this run's scope, adoption of a
/// real directory already at the linked name, and the declared link. An
/// underivable scope shares nothing and is not an error — a kernel that will
/// not name its boot or mount namespace leaves the child with the private
/// registry it always had ([ADR-0108]).
///
/// [ADR-0108]: ../../docs/decisions/ADR-0108-share-the-child-peer-registry-across-sessions.md
fn share_registry(
    context: &AppContext,
    account: &Identifier,
    agent: &Agent,
) -> Result<(), AppError> {
    use crate::adapters::host;
    use crate::domain::{namespace::Kind, peers::Scope};

    let scope = host::boot_id().and_then(|boot| {
        host::namespace_link(Kind::Mount).and_then(|link| Scope::derive(&boot, &link))
    });
    let Some(scope) = scope else {
        // A warning, not a debug line: the slice contract promises the
        // degradation is visible at the normal threshold ([ADR-0108]).
        tracing::warn!(
            op = "share_peer_registry",
            status = "degraded",
            "no peer scope could be derived, so this session keeps a private registry"
        );
        return Ok(());
    };
    let paths = context.paths();
    let state = paths.state();
    let registry = paths.peer_registry(scope.boot(), scope.namespace());
    guard::ensure_directory(state, &registry)?;
    let link = paths.session_registry_link(account, agent.namespace().id(), agent.id());
    adopt_registry(&link, &registry, &paths.peers())?;
    guard::ensure_link(state, &link, &registry)
}

/// Frees the linked name of what an earlier launch left there.
///
/// Two occupants are the wrapper's own to clear. A real `sessions/` directory
/// is a session made before the share, so its entries move into the registry;
/// the entries are the child's pid-keyed registrations, so a rename that
/// replaces a same-named entry is safe — liveness is decided at read time, and
/// a process id names one live process at most. A link into the registry tree
/// that no longer targets this scope is the wrapper's own writing for an
/// earlier boot; left in place the guard would refuse it on every later
/// launch, permanently degrading the share, so it is removed and relinked.
/// Anything else — including a link pointing outside the registry tree — is
/// left for the guard's refusal, which names what it found. A child racing
/// the move can recreate the directory between the removal
/// and the link; the link step then refuses once and the next launch retries
/// clean.
fn adopt_registry(link: &Path, registry: &Path, peers: &Path) -> Result<(), AppError> {
    use crate::adapters::filesystem::SystemFileSystem;

    let occupied = |error: std::io::Error| {
        AppError::new(
            crate::error::ErrorKind::Io,
            Diagnostic::new(
                "the session registry could not be adopted",
                "the peer registry share",
                error.to_string(),
                "remove the session directory's `sessions` entry, then run this again",
            ),
        )
    };
    match SystemFileSystem::look(link).map_err(occupied)? {
        Some(facts) if facts.directory => {}
        Some(facts) if facts.symlink => {
            let target = std::fs::read_link(link).map_err(occupied)?;
            if target != registry && target.starts_with(peers) {
                std::fs::remove_file(link).map_err(occupied)?;
            }
            return Ok(());
        }
        // Nothing at the name is the guard's to create, and any other
        // occupant is left for its refusal.
        _ => return Ok(()),
    }
    for entry in SystemFileSystem::dir_entries(link).map_err(occupied)? {
        std::fs::rename(link.join(&entry), registry.join(&entry)).map_err(occupied)?;
    }
    std::fs::remove_dir(link).map_err(occupied)
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
