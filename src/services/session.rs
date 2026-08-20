//! Resolving this run's account and profile selection into paths.
//!
//! This module is for turning a selection into paths and preparing them on
//! first use. It is not for authentication, and it puts nothing on the child's
//! argument vector.

use std::path::{Path, PathBuf};

use crate::{
    adapters::terminal::Terminal as _,
    context::AppContext,
    domain::{checks::Check, identifier::Identifier, terminal::Terminal, witness::Marker},
    error::{AppError, Diagnostic},
    services::storage::{atomic, guard},
};

pub(crate) mod gc;

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
    // Additive, like the registry share below: a launch that cannot record
    // its witness leaves the session judged unknown — kept, never collected —
    // so degrading is honest where refusing the exec would not be
    // ([ADR-0110]).
    //
    // [ADR-0110]: ../../docs/decisions/ADR-0110-record-the-terminal-witness-at-launch.md
    if let Err(error) = record_witness(context, account, terminal) {
        tracing::warn!(
            op = "record_witness",
            status = "degraded",
            "this session's terminal witness was not recorded: {}",
            error.diagnostic().why
        );
    }
    // Additive, so a failure degrades to an unshared launch instead of
    // refusing the exec ([ADR-0108]): the child's peer discovery is a
    // convenience, and a damaged shared registry must not stop every
    // terminal of every account from launching.
    //
    // [ADR-0108]: ../../docs/decisions/ADR-0108-share-the-child-peer-registry-across-sessions.md
    if let Err(error) = share_registry(context, account, terminal) {
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
        terminal = terminal.id().as_str(),
        namespace = namespace.as_str(),
        path = %directory.display(),
        "this run will use the session directory at {}",
        directory.display()
    );
    Ok(directory)
}

/// Records the witness of this terminal's name beside its session directory.
///
/// Write-if-changed: repeated launches of one terminal produce identical
/// bytes, so the common relaunch touches nothing, and only a changed fact —
/// a new boot, a rung change on a reused slot — replaces the record
/// ([ADR-0110]).
///
/// [ADR-0110]: ../../docs/decisions/ADR-0110-record-the-terminal-witness-at-launch.md
fn record_witness(
    context: &AppContext,
    account: &Identifier,
    terminal: &Terminal,
) -> Result<(), AppError> {
    let degraded = |why: &str| {
        AppError::new(
            crate::error::ErrorKind::Io,
            Diagnostic::new(
                "the terminal witness could not be recorded",
                "the session witness record",
                why.to_owned(),
                "the session will be judged unknown and never collected",
            ),
        )
    };
    let marker = Marker::from_terminal(terminal, crate::adapters::host::boot_id())
        .ok_or_else(|| degraded("the device path is not valid UTF-8"))?;
    let bytes = marker
        .to_bytes()
        .ok_or_else(|| degraded("the record did not serialize"))?;
    let paths = context.paths();
    let path = paths.session_witness(account, terminal.namespace().id(), terminal.id());
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
    terminal: &Terminal,
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
    let link = paths.session_registry_link(account, terminal.namespace().id(), terminal.id());
    adopt_registry(&link, &registry, &paths.peers())?;
    guard::ensure_link(state, &link, &registry)
}

/// Frees the linked name of what an earlier launch left there.
///
/// Two occupants are the wrapper's own to clear. A real `sessions/` directory
/// is a terminal used before the share, so its entries move into the registry;
/// the entries are the child's pid-keyed registrations, so a rename that
/// replaces a same-named entry is safe — liveness is decided at read time, and
/// a process id names one live process at most. A link into the registry tree
/// that no longer targets this scope is the wrapper's own writing for an
/// earlier boot; left in place the guard would refuse it on every later
/// launch, permanently degrading the share, so it is removed and relinked.
/// Anything else — including a link pointing outside the registry tree — is
/// left for the guard's refusal, which names what it found. A child of this
/// terminal racing the move can recreate the directory between the removal
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
