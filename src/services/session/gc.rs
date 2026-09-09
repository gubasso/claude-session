//! Judging and collecting per-agent session directories.
//!
//! This module gathers what the kernel says about each recorded agent and hands
//! the pure judgment to `domain::witness`. A session is one running agent, and
//! the tree is the wrapper's own, so a directory is kept only while its agent
//! is still running and everything else is garbage ([ADR-0112], [ADR-0113]).
//!
//! [ADR-0112]: ../../../docs/decisions/ADR-0112-keep-only-the-session-proven-live.md
//! [ADR-0113]: ../../../docs/decisions/ADR-0113-key-a-session-to-its-running-agent.md

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use crate::{
    adapters::{filesystem::SystemFileSystem, host},
    context::AppContext,
    domain::{
        identifier::Identifier,
        namespace::Kind,
        paths::XdgPaths,
        peers::Scope,
        registration::{Registration, Subject},
        witness::{self, Ground, Marker, Observed, Process, Verdict},
    },
    error::{AppError, Diagnostic, ErrorKind},
    services::storage::guard,
};

/// One session directory and what this run could say about it.
#[derive(Clone, Debug)]
pub(crate) struct SessionFinding {
    /// The account the session belongs to.
    pub(crate) account: Identifier,
    /// The namespace directory the agent's process identifier is unique inside.
    pub(crate) namespace: Identifier,
    /// The session directory name.
    pub(crate) session: Identifier,
    /// The liveness verdict, which is the ground's projection.
    pub(crate) verdict: Verdict,
    /// Why the verdict was reached, which is what a report explains.
    pub(crate) ground: Ground,
    /// The process the record names, absent when no record could be read.
    pub(crate) pid: Option<u32>,
    /// The name the child registered for that process, absent when no
    /// registration in its resolved registry names it ([ADR-0114]).
    ///
    /// [ADR-0114]: ../../../docs/decisions/ADR-0114-name-a-reported-session-as-the-child-does.md
    pub(crate) name: Option<String>,
    /// The child's working directory, when the registration carries one.
    pub(crate) working_directory: Option<String>,
    /// The child's own status word, carried verbatim.
    pub(crate) claude_status: Option<String>,
    /// Whether this session's registrations live in this run's own peer scope.
    ///
    /// Naming a session and reaching it stopped being one answer ([ADR-0123]).
    pub(crate) reachable: bool,
    /// Whether the reader is running inside this session's agent.
    pub(crate) current: bool,
    /// The session directory.
    pub(crate) path: PathBuf,
}

impl SessionFinding {
    /// Reports whether a filter naming `subject` selects this row.
    pub(crate) fn answers_to(&self, subject: &Subject) -> bool {
        self.session.as_str() == subject.as_str() || self.name.as_deref() == Some(subject.as_str())
    }
}

/// What one collection removed.
#[derive(Clone, Debug)]
pub(crate) struct Collection {
    /// The sessions removed, in survey order.
    pub(crate) removed: Vec<SessionFinding>,
    /// The namespace directories removed because nothing was left inside.
    pub(crate) pruned_namespaces: usize,
}

/// What this run observed once, shared across every judgment.
struct Observatory {
    namespace: Option<Identifier>,
    boot: Option<String>,
    /// This process's ancestry, which is how a row is marked as the reader's
    /// own: a command is never an agent, so the only session it is "in" is an
    /// agent it descends from.
    lineage: Vec<(u32, u64)>,
    /// This run's own peer registry, when its scope can be derived.
    own_registry: Option<PathBuf>,
}

impl Observatory {
    fn capture(paths: &XdgPaths) -> Self {
        Self {
            namespace: host::namespace(Kind::Pid).map(|space| space.id().clone()),
            boot: host::boot_id(),
            lineage: host::lineage(),
            own_registry: own_registry(paths),
        }
    }

    /// Gathers the per-marker facts the pure judgment needs.
    ///
    /// A process that has exited is `Absent` even while the kernel still lists
    /// it, which it does until something reaps it. The agent ended when it
    /// exited, so a session waiting on a parent that never calls `wait` is over
    /// too; reading the run state is what tells that apart from a running
    /// agent, because `kill(pid, 0)` answers for both alike ([ADR-0113]).
    fn observe(&self, marker: &Marker) -> Observed {
        let process = if SystemFileSystem::process_is_live(marker.pid()) {
            match host::process_stat(marker.pid()) {
                Some(stat) if stat.exited() => Process::Absent,
                Some(stat) => Process::Running {
                    started: stat.started(),
                },
                None => Process::Unreadable,
            }
        } else {
            Process::Absent
        };
        Observed {
            namespace: self.namespace.clone(),
            boot: self.boot.clone(),
            process,
        }
    }

    /// Reports whether the reader descends from the recorded process.
    ///
    /// Only the ancestry: the record still has to be one this run can place,
    /// which is the judgment's business and the caller's to pair with this.
    fn descends_from(&self, marker: &Marker) -> bool {
        self.lineage
            .iter()
            .any(|(pid, started)| *pid == marker.pid() && *started == marker.started())
    }
}

/// Judges one session directory, with or without a readable record.
///
/// The single owner of "what is this directory", so the survey and the
/// re-judgment inside the removal lock cannot reach different answers from the
/// same bytes. A directory carrying no readable record is `Unrecorded` rather
/// than skipped: it is still a directory in a tree the wrapper owns, and the
/// collector has to be able to act on it.
///
/// A record filed under a namespace directory it does not name witnesses
/// nothing about that directory, so it is treated as no record at all rather
/// than judged.
fn judge_directory(
    paths: &XdgPaths,
    observatory: &Observatory,
    account: &Identifier,
    namespace: &Identifier,
    session: &Identifier,
) -> (Ground, Option<Marker>) {
    let marker = read_marker(&paths.session_witness(account, namespace, session))
        .filter(|it| it.namespace() == namespace);
    let ground = marker.as_ref().map_or(Ground::Unrecorded, |it| {
        witness::judge(it, &observatory.observe(it))
    });
    (ground, marker)
}

/// Surveys every session directory of every account, judging each one.
///
/// Findings are ordered by account, then namespace, then session, so two
/// surveys of one unchanged tree report identically.
pub(crate) fn survey(context: &AppContext) -> Result<Vec<SessionFinding>, AppError> {
    let paths = context.paths();
    let observatory = Observatory::capture(paths);
    let mut registries = Registries::default();
    let mut findings = Vec::new();
    for account in identifier_directories(&paths.accounts())? {
        findings.extend(survey_account(
            paths,
            &observatory,
            &mut registries,
            &account,
        )?);
    }
    Ok(findings)
}

/// Surveys one account's session directories.
fn survey_account(
    paths: &XdgPaths,
    observatory: &Observatory,
    registries: &mut Registries,
    account: &Identifier,
) -> Result<Vec<SessionFinding>, AppError> {
    let mut findings = Vec::new();
    for namespace in identifier_directories(&paths.account_sessions(account))? {
        for session in identifier_directories(&paths.account_namespace(account, &namespace))? {
            let (ground, marker) =
                judge_directory(paths, observatory, account, &namespace, &session);
            let path = paths.account_session(account, &namespace, &session);
            let registry = session_registry(paths, &path.join("sessions"));
            let registration = marker.as_ref().and_then(|marker| {
                registry
                    .as_ref()
                    .and_then(|registry| described(registries.load(registry), marker))
            });
            findings.push(SessionFinding {
                path,
                account: account.clone(),
                namespace: namespace.clone(),
                session,
                verdict: ground.verdict(),
                ground,
                pid: marker.as_ref().map(Marker::pid),
                name: registration.map(|it| it.name().to_owned()),
                working_directory: registration
                    .and_then(Registration::working_directory)
                    .map(str::to_owned),
                claude_status: registration
                    .and_then(Registration::child_status)
                    .map(str::to_owned),
                reachable: registry.is_some()
                    && registry.as_ref() == observatory.own_registry.as_ref(),
                // The ancestry is read in this run's namespace and under this
                // boot, so a record naming neither cannot be the agent this
                // reader is inside however its pair compares. `Running` is the
                // ground that establishes both, which is why the mark is taken
                // against the judgment rather than beside it ([ADR-0113]).
                current: matches!(ground, Ground::Running)
                    && marker
                        .as_ref()
                        .is_some_and(|it| observatory.descends_from(it)),
            });
        }
    }
    Ok(findings)
}

/// Removes every collectable finding, then prunes what the removals emptied.
///
/// Per account, under that account's write lock, so a removal never interleaves
/// with an `account remove` destroying the same scope. Each directory is
/// re-validated by the guard before deletion — a removal that followed a
/// symbolic link would delete something the user never named — and its witness
/// goes after it, so a crash between the two leaves an orphan record rather
/// than a directory nothing accounts for. A namespace directory is removed only
/// once nothing but orphan witness records is left inside it ([ADR-0112]).
///
/// [ADR-0112]: ../../../docs/decisions/ADR-0112-keep-only-the-session-proven-live.md
pub(crate) fn collect(
    context: &AppContext,
    collectable: &[SessionFinding],
) -> Result<Collection, AppError> {
    let paths = context.paths();
    let state = paths.state();
    let observatory = Observatory::capture(paths);
    let mut removed = Vec::new();
    let mut pruned_namespaces = 0;
    let mut accounts: Vec<&Identifier> =
        collectable.iter().map(|finding| &finding.account).collect();
    accounts.dedup();
    for account in accounts {
        let _lock = crate::services::account::hold(context, account)?;
        let mut namespaces: Vec<&Identifier> = Vec::new();
        for finding in collectable
            .iter()
            .filter(|finding| &finding.account == account)
        {
            debug_assert!(finding.verdict.collectable());
            let witness_path =
                paths.session_witness(&finding.account, &finding.namespace, &finding.session);
            // The confirmation prompt sat between the survey and this lock, and
            // a directory can become accountable in that window — a launch that
            // recorded the witness this run could not read. Only a judgment
            // re-taken inside the critical section is current enough to act on,
            // so anything now provably live is left standing ([ADR-0112]).
            let (ground, _) = judge_directory(
                paths,
                &observatory,
                &finding.account,
                &finding.namespace,
                &finding.session,
            );
            if !ground.verdict().collectable() {
                continue;
            }
            guard::validate(state, &finding.path, guard::Expected::Directory)?;
            crate::services::account::remove_tree(&finding.path)
                .map_err(|error| removal_error(&finding.path, &error))?;
            match std::fs::remove_file(&witness_path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(removal_error(&witness_path, &error)),
            }
            if !namespaces.contains(&&finding.namespace) {
                namespaces.push(&finding.namespace);
            }
            removed.push(finding.clone());
        }
        for namespace in namespaces {
            if prune_namespace(&paths.account_namespace(account, namespace))? {
                pruned_namespaces += 1;
            }
        }
    }
    tracing::info!(
        op = "collect_sessions",
        status = "ok",
        removed = removed.len(),
        pruned_namespaces,
        "collected {} session directories",
        removed.len()
    );
    Ok(Collection {
        removed,
        pruned_namespaces,
    })
}

/// Removes one account's provably dead sessions, at launch.
///
/// A session is one agent run, so a directory is created every time the wrapper
/// launches and abandoned the moment that agent exits. Without this the tree
/// would grow by one directory per launch forever, and the explicit verb would
/// be the only thing holding it back — a maintenance chore the design would be
/// imposing rather than a choice a user makes ([ADR-0113]).
///
/// Only `Dead` is swept, never `Unknown`. A launch is not the place to act on
/// what could not be decided: the reader is not watching, and an undecidable
/// directory is exactly the one a person should see named before it goes.
///
/// [ADR-0113]: ../../../docs/decisions/ADR-0113-key-a-session-to-its-running-agent.md
pub(crate) fn sweep(context: &AppContext, account: &Identifier) -> Result<usize, AppError> {
    let observatory = Observatory::capture(context.paths());
    let mut registries = Registries::default();
    let dead: Vec<SessionFinding> =
        survey_account(context.paths(), &observatory, &mut registries, account)?
            .into_iter()
            .filter(|finding| matches!(finding.verdict, Verdict::Dead))
            .collect();
    if dead.is_empty() {
        return Ok(0);
    }
    Ok(collect(context, &dead)?.removed.len())
}

/// Removes a namespace directory once nothing meaningful is left inside.
///
/// An orphan witness record — one whose session directory is gone, which a
/// crash between the two removals above can leave — does not keep the
/// directory alive: it witnesses nothing. Anything else does, so the directory
/// stays. Returns whether the directory was removed.
fn prune_namespace(namespace: &Path) -> Result<bool, AppError> {
    let entries = SystemFileSystem::dir_entries(namespace)
        .map_err(|error| removal_error(namespace, &error))?;
    let mut orphans = Vec::new();
    for entry in &entries {
        let Some(name) = entry.to_str() else {
            return Ok(false);
        };
        let orphan = name
            .strip_prefix('.')
            .and_then(|rest| rest.strip_suffix(".witness.json"))
            .is_some_and(|session| {
                matches!(SystemFileSystem::look(&namespace.join(session)), Ok(None))
            });
        if orphan {
            orphans.push(namespace.join(name));
        } else {
            return Ok(false);
        }
    }
    for orphan in orphans {
        match std::fs::remove_file(&orphan) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(removal_error(&orphan, &error)),
        }
    }
    match std::fs::remove_dir(namespace) {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(removal_error(namespace, &error)),
    }
}

/// Lists a directory's identifier-named, non-symlink subdirectories, sorted.
///
/// An absent parent is an empty answer, because an account with no sessions
/// simply has no sessions. A name outside the identifier grammar is not the
/// wrapper's writing and is left unread.
fn identifier_directories(parent: &Path) -> Result<Vec<Identifier>, AppError> {
    if matches!(SystemFileSystem::look(parent), Ok(None)) {
        return Ok(Vec::new());
    }
    let mut names: Vec<Identifier> = SystemFileSystem::dir_entries(parent)
        .map_err(|error| removal_error(parent, &error))?
        .into_iter()
        .filter_map(|entry| entry.to_str().and_then(|name| name.parse().ok()))
        .filter(|name: &Identifier| {
            matches!(
                SystemFileSystem::look(&parent.join(name.as_str())),
                Ok(Some(facts)) if facts.directory && !facts.symlink
            )
        })
        .collect();
    names.sort_by(|left, right| left.as_str().cmp(right.as_str()));
    Ok(names)
}

fn own_registry(paths: &XdgPaths) -> Option<PathBuf> {
    let scope = host::boot_id().and_then(|boot| {
        host::namespace_link(Kind::Mount).and_then(|link| Scope::derive(&boot, &link))
    })?;
    Some(paths.peer_registry(scope.boot(), scope.namespace()))
}

/// Resolves one session's own registry from the `sessions` link a launch wrote.
fn session_registry(paths: &XdgPaths, link: &Path) -> Option<PathBuf> {
    let facts = SystemFileSystem::look(link).ok()??;
    if !facts.symlink {
        return None;
    }
    let target = std::fs::read_link(link).ok()?;
    paths.adopted_registry(&target)
}

/// The registration that is provably this agent's, or none if two are.
fn described<'a>(registrations: &'a [Registration], marker: &Marker) -> Option<&'a Registration> {
    let mut matches = registrations
        .iter()
        .filter(|it| it.names(marker.pid(), marker.started()));
    let registration = matches.next()?;
    matches.next().is_none().then_some(registration)
}

/// The registries one survey read, so many sessions of one scope read that
/// directory once.
#[derive(Default)]
struct Registries {
    loaded: HashMap<PathBuf, Vec<Registration>>,
}

impl Registries {
    fn load(&mut self, registry: &Path) -> &[Registration] {
        self.loaded.entry(registry.to_owned()).or_insert_with(|| {
            SystemFileSystem::dir_entries(registry).map_or_else(
                |_| Vec::new(),
                |entries| {
                    entries
                        .iter()
                        .filter_map(|entry| read_registration(&registry.join(entry)))
                        .collect()
                },
            )
        })
    }
}

/// Reads one registration, treating anything unreadable as no registration.
///
/// Guarded like the witness beside it, and for the same reason: bytes reached
/// through a symbolic link or owned by another user must not answer for a
/// session in a tree this wrapper owns ([ADR-0061]). The mode is not judged —
/// the child writes these, and its choice of mode is its own.
///
/// [ADR-0061]: ../../../docs/decisions/ADR-0061-protect-storage-from-accidental-local-drift.md
fn read_registration(path: &Path) -> Option<Registration> {
    let facts = SystemFileSystem::look(path).ok()??;
    if facts.symlink || !facts.regular || facts.uid != rustix::process::getuid().as_raw() {
        return None;
    }
    std::fs::read(path)
        .ok()
        .and_then(|bytes| Registration::from_bytes(&bytes))
}

/// Reads one witness record, treating anything unreadable as no record.
///
/// A record reached through a symbolic link, or one another user owns, is
/// refused before it is read: either would let bytes the wrapper never wrote
/// answer for a directory it owns, the drift [ADR-0061] guards the tree
/// against. The mode is not judged here — a widened mode leaks the record but
/// does not forge it, and the launch path's guard settles it back to `0600`.
///
/// [ADR-0061]: ../../../docs/decisions/ADR-0061-protect-storage-from-accidental-local-drift.md
fn read_marker(path: &Path) -> Option<Marker> {
    let facts = SystemFileSystem::look(path).ok()??;
    if facts.symlink || !facts.regular || facts.uid != rustix::process::getuid().as_raw() {
        return None;
    }
    std::fs::read(path)
        .ok()
        .and_then(|bytes| Marker::from_bytes(&bytes))
}

fn removal_error(path: &Path, error: &dyn std::fmt::Display) -> AppError {
    AppError::new(
        ErrorKind::Io,
        Diagnostic::new(
            "a session directory operation failed",
            path.display().to_string(),
            error.to_string(),
            "inspect the path, then run claude-session session list again",
        ),
    )
}
