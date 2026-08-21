//! Judging and collecting per-terminal session directories.
//!
//! This module gathers what the kernel says about each recorded witness and
//! hands the pure judgment to `domain::witness`. The session tree is the
//! wrapper's own, so a directory is kept only while this run can prove its
//! terminal is still there, and everything else is garbage ([ADR-0112]).
//!
//! [ADR-0112]: ../../../docs/decisions/ADR-0112-keep-only-the-session-proven-live.md

use std::path::{Path, PathBuf};

use crate::{
    adapters::{filesystem::SystemFileSystem, host, terminal::Terminal as _},
    context::AppContext,
    domain::{
        identifier::Identifier,
        namespace::Kind,
        paths::XdgPaths,
        terminal::Source,
        witness::{self, Ground, LeaderObservation, Marker, Observed, Recorded, Verdict},
    },
    error::{AppError, Diagnostic, ErrorKind},
    services::storage::guard,
};

/// One session directory and what this run could say about it.
#[derive(Clone, Debug)]
pub(crate) struct SessionFinding {
    /// The account the session belongs to.
    pub(crate) account: Identifier,
    /// The namespace directory the terminal name is unique inside.
    pub(crate) namespace: Identifier,
    /// The terminal directory name.
    pub(crate) terminal: Identifier,
    /// The liveness verdict, which is the ground's projection.
    pub(crate) verdict: Verdict,
    /// Why the verdict was reached, which is what a report explains.
    pub(crate) ground: Ground,
    /// The recorded rung, absent when no witness could be read.
    pub(crate) source: Option<Source>,
    /// The terminal the record names, when it names one a reader would know:
    /// the device path, or the leader's process id. Absent for a record that
    /// names no terminal, the alias included.
    pub(crate) named: Option<String>,
    /// Whether this is the session the run doing the reporting belongs to.
    pub(crate) current: bool,
    /// The session directory.
    pub(crate) path: PathBuf,
}

/// What one `clean` removed.
#[derive(Clone, Debug)]
pub(crate) struct Collection {
    /// The sessions removed, in survey order.
    pub(crate) removed: Vec<SessionFinding>,
    /// The namespace directories removed because nothing was left inside.
    pub(crate) pruned_namespaces: usize,
}

/// What this run observed once, shared across every judgment.
struct Observatory {
    mount: Option<Identifier>,
    pid: Option<Identifier>,
    boot: Option<String>,
}

impl Observatory {
    fn capture() -> Self {
        Self {
            mount: host::namespace(Kind::Mount).map(|space| space.id().clone()),
            pid: host::namespace(Kind::Pid).map(|space| space.id().clone()),
            boot: host::boot_id(),
        }
    }

    /// Gathers the per-marker facts the pure judgment needs.
    fn observe(&self, marker: &Marker) -> Observed {
        let namespace = match marker.witness().namespace_kind() {
            Kind::Mount => self.mount.clone(),
            Kind::Pid => self.pid.clone(),
        };
        let device_present = match marker.witness() {
            Recorded::Tty { device } => SystemFileSystem::look(Path::new(device))
                .ok()
                .map(|found| found.is_some()),
            Recorded::SessionLeader { .. } => None,
        };
        let leader = match marker.witness() {
            Recorded::SessionLeader { sid, .. } => {
                if SystemFileSystem::process_is_live(*sid) {
                    host::process_started(*sid).map_or(LeaderObservation::Unreadable, |started| {
                        LeaderObservation::Running { started }
                    })
                } else {
                    LeaderObservation::Absent
                }
            }
            Recorded::Tty { .. } => LeaderObservation::Absent,
        };
        Observed {
            namespace,
            boot: self.boot.clone(),
            device_present,
            leader,
        }
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
    terminal: &Identifier,
) -> (Ground, Option<Marker>) {
    let marker = read_marker(&paths.session_witness(account, namespace, terminal))
        .filter(|it| it.namespace() == namespace);
    let ground = marker.as_ref().map_or(Ground::Unrecorded, |it| {
        witness::judge(it, &observatory.observe(it))
    });
    (ground, marker)
}

/// Surveys every session directory of every account, judging each one.
///
/// Findings are ordered by account, then namespace, then terminal, so two
/// surveys of one unchanged tree report identically.
pub(crate) fn survey(context: &AppContext) -> Result<Vec<SessionFinding>, AppError> {
    let observatory = Observatory::capture();
    // Which row is the reader's own pane. A run that cannot name its terminal
    // marks none, which costs the report a note and nothing else.
    let here = context
        .adapters()
        .terminal()
        .identity()
        .ok()
        .flatten()
        .map(|terminal| (terminal.namespace().id().clone(), terminal.id().clone()));
    let paths = context.paths();
    let mut findings = Vec::new();
    for account in identifier_directories(&paths.accounts())? {
        for namespace in identifier_directories(&paths.account_sessions(&account))? {
            for terminal in identifier_directories(&paths.account_namespace(&account, &namespace))?
            {
                let (ground, marker) =
                    judge_directory(paths, &observatory, &account, &namespace, &terminal);
                let current = here
                    .as_ref()
                    .is_some_and(|(space, pane)| space == &namespace && pane == &terminal);
                findings.push(SessionFinding {
                    path: paths.account_session(&account, &namespace, &terminal),
                    account: account.clone(),
                    namespace: namespace.clone(),
                    terminal,
                    verdict: ground.verdict(),
                    ground,
                    source: marker.as_ref().map(|it| it.witness().source()),
                    named: marker.as_ref().and_then(|it| named(it.witness())),
                    current,
                });
            }
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
    let observatory = Observatory::capture();
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
                paths.session_witness(&finding.account, &finding.namespace, &finding.terminal);
            // The confirmation prompt sat between the survey and this lock,
            // and a slot can be reborn in that window — a reopened tty, a
            // relaunched leader, a launch that recorded the witness this run
            // could not read. Only a judgment re-taken inside the critical
            // section is current enough to act on, so anything that has become
            // provably live is left standing ([ADR-0112]).
            let (ground, _) = judge_directory(
                paths,
                &observatory,
                &finding.account,
                &finding.namespace,
                &finding.terminal,
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

/// Removes a namespace directory once nothing meaningful is left inside.
///
/// An orphan witness record — one whose terminal directory is gone, which a
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
            .is_some_and(|terminal| {
                matches!(SystemFileSystem::look(&namespace.join(terminal)), Ok(None))
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
/// tree simply has no sessions. A name outside the identifier grammar is not
/// the wrapper's writing and is left unread.
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

/// Names the terminal a record witnesses, in the words its reader would use.
///
/// The device path for a pane and the leader's process id for a run that had
/// none. The start time is left out: it is what tells one process from a later
/// one wearing its id, which is a judgment input rather than a name anybody
/// would recognise.
///
/// A record naming the alias every process shares names no terminal, so it
/// gets no name. That is read from the record itself rather than from the
/// verdict's ground, because a namespace this run cannot decide — foreign, or
/// unplaced — never reaches the alias ground and would otherwise report a
/// device standing for every pane at once.
fn named(witness: &Recorded) -> Option<String> {
    match witness {
        Recorded::Tty { device } if device == witness::ALIAS => None,
        Recorded::Tty { device } => Some(device.clone()),
        Recorded::SessionLeader { sid, .. } => Some(format!("process group {sid}")),
    }
}

fn removal_error(path: &Path, error: &dyn std::fmt::Display) -> AppError {
    AppError::new(
        ErrorKind::Io,
        Diagnostic::new(
            "a session directory operation failed",
            path.display().to_string(),
            error.to_string(),
            "inspect the path, then run claude-session-rs session list again",
        ),
    )
}
