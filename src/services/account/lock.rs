//! The credential lock scope: steps 1 through 3 and step 9 of the sequence.
//!
//! `super::super::storage::atomic` owns steps 4 through 8 and disclaims these,
//! because exactly one lock scope exists and it belongs with the subsystem that
//! writes the pair it guards. That pair is `oauth-token` and `auth-mode.json`:
//! each login mints a new secret, so a reordered rename could persist a revoked
//! one, and the two files carry one invariant a single rename cannot express.
//!
//! Two writers share the scope. `account login --token` rotates the pair, and
//! `account remove` destroys the scope itself. A launch takes no lock, because
//! it only reads.

use std::{
    collections::HashMap,
    fs,
    os::unix::fs::OpenOptionsExt as _,
    path::{Path, PathBuf},
    sync::{LazyLock, Mutex, MutexGuard, PoisonError},
    time::{Duration, Instant},
};

use crate::{
    error::{AppError, Diagnostic, ErrorKind},
    services::storage::guard,
};

/// How long acquisition blocks, and how often it retries within that budget.
///
/// Separated from `acquire` so the retry policy is exercised by a unit test
/// that neither touches a filesystem nor waits for a real deadline. A test-only
/// environment override was the alternative and was rejected: it would be an
/// undocumented key that the configuration table, the strict unknown-key
/// rejection, and the `CLAUDE_SESSION_RS_` scrub would each have to acknowledge,
/// where a parameter costs nothing and is visible in the signature.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Deadline {
    budget: Duration,
    poll: Duration,
}

impl Deadline {
    /// The deadline every production acquisition uses.
    ///
    /// The critical section is two renames and two syncs — the child probe runs
    /// before the lock is taken, so no spawn happens inside it — which makes a
    /// real contention window milliseconds long. Two seconds is three orders of
    /// magnitude of headroom and still short enough that the contended path is
    /// a test rather than a hang.
    pub(crate) const STANDARD: Self = Self {
        budget: Duration::from_secs(2),
        poll: Duration::from_millis(25),
    };
}

/// A held credential lock. Dropping it performs step 9.
///
/// Field order is release order, and it is load-bearing: the file lock must be
/// released before the in-process mutex. Released the other way round, a
/// sibling thread could win the mutex and then block on a file lock this thread
/// still holds, spending its whole budget against its own process.
pub(crate) struct CredentialLock {
    // Both are held for their `Drop` rather than read. The file's descriptor is
    // the lock, and closing it is the release; the guard is step 1's exclusion.
    #[allow(
        dead_code,
        reason = "the descriptor is the lock, and closing it releases"
    )]
    file: fs::File,
    #[allow(dead_code, reason = "the guard is the in-process half of the scope")]
    scope: MutexGuard<'static, ()>,
    path: PathBuf,
}

impl std::fmt::Debug for CredentialLock {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CredentialLock")
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

/// Acquires one account's credential lock, or fails at the deadline.
///
/// The sentinel lives beside the account rather than inside it, which is what
/// makes the exclusion real: its inode outlives the removal, so a login
/// arriving at any point during one waits on the same file the remover holds
/// instead of creating a second inode nobody is excluded by
/// ([ADR-0087](../../../docs/decisions/ADR-0087-keep-the-credential-lock-beside-the-account.md)).
///
/// The file is opened once, before the retry loop, and that descriptor is held
/// for the whole wait.
pub(crate) fn acquire(
    state_root: &Path,
    lock_path: &Path,
    deadline: Deadline,
) -> Result<CredentialLock, AppError> {
    let collection = lock_path.parent().ok_or_else(|| {
        AppError::new(
            ErrorKind::Internal,
            Diagnostic::new(
                "the credential lock has no parent directory",
                lock_path.display().to_string(),
                "a lock path is always inside the account collection",
                "report this wrapper bug",
            ),
        )
    })?;
    guard::ensure_directory(state_root, collection)?;
    // No guard::validate on the sentinel itself: a lock file carries no
    // security check and is never swept, because it holds no content and a
    // permission fault on it must not be reported as a credential defect.
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .open(lock_path)
        .map_err(|error| io_error(lock_path, &error))?;
    let mutex = scope_mutex(lock_path);
    wait_for(deadline, lock_path, || {
        // The mutex is taken inside the loop and dropped on contention, so a
        // caller with a shorter budget fails fast instead of starving behind
        // one with a longer one.
        let scope = mutex.lock().unwrap_or_else(PoisonError::into_inner);
        match file.try_lock() {
            Ok(()) => Attempt::Acquired(scope),
            Err(fs::TryLockError::WouldBlock) => Attempt::Contended,
            Err(fs::TryLockError::Error(error)) => Attempt::Failed(error),
        }
    })
    .map(|scope| CredentialLock {
        file,
        scope,
        path: lock_path.to_path_buf(),
    })
}

/// One acquisition attempt's outcome.
enum Attempt<T> {
    Acquired(T),
    Contended,
    Failed(std::io::Error),
}

/// Retries `attempt` until it succeeds or the budget is spent.
///
/// A zero budget still makes exactly one attempt, so "do not wait" and "never
/// try" stay distinguishable.
fn wait_for<T>(
    deadline: Deadline,
    lock_path: &Path,
    mut attempt: impl FnMut() -> Attempt<T>,
) -> Result<T, AppError> {
    let started = Instant::now();
    loop {
        match attempt() {
            Attempt::Acquired(value) => return Ok(value),
            // A file lock the platform cannot take at all is not contention,
            // and saying so is what tells the user that retrying will not help.
            Attempt::Failed(error) => return Err(io_error(lock_path, &error)),
            Attempt::Contended => {
                if started.elapsed() >= deadline.budget {
                    return Err(busy(lock_path, deadline.budget));
                }
                std::thread::sleep(deadline.poll);
            }
        }
    }
}

/// Interns one mutex per lock-file path, for the process's lifetime.
///
/// Step 1 of the sequence exists because a file lock is per open file
/// description and therefore cannot exclude a second thread of the same
/// process. Keyed by path rather than by account name, since one process can
/// address two state roots and the path is what identifies the scope.
///
/// The mutex is leaked because the standard library has no owned guard, and a
/// held lock has to outlive the call that took it. The set is bounded by the
/// distinct accounts one process touches, which in production is one.
fn scope_mutex(lock_path: &Path) -> &'static Mutex<()> {
    static SCOPES: LazyLock<Mutex<HashMap<PathBuf, &'static Mutex<()>>>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));
    let mut scopes = SCOPES.lock().unwrap_or_else(PoisonError::into_inner);
    scopes
        .entry(lock_path.to_path_buf())
        .or_insert_with(|| Box::leak(Box::new(Mutex::new(()))))
}

fn busy(path: &Path, budget: Duration) -> AppError {
    AppError::new(
        ErrorKind::LockBusy,
        Diagnostic::new(
            "the account credential lock is held",
            path.display().to_string(),
            format!(
                "another run held it for the whole {} millisecond wait",
                budget.as_millis()
            ),
            concat!(
                "wait for the other `claude-session-rs account login` or ",
                "`account remove` to finish, then retry"
            ),
        ),
    )
}

fn io_error(path: &Path, error: &std::io::Error) -> AppError {
    AppError::new(
        ErrorKind::Io,
        Diagnostic::new(
            "the account credential lock is unusable",
            path.display().to_string(),
            error.to_string(),
            "check that the account directory is on a filesystem supporting advisory locks",
        ),
    )
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    fn deadline(budget_ms: u64, poll_ms: u64) -> Deadline {
        Deadline {
            budget: Duration::from_millis(budget_ms),
            poll: Duration::from_millis(poll_ms),
        }
    }

    #[test]
    fn an_uncontended_attempt_succeeds_without_waiting() {
        let mut calls = 0_u32;
        wait_for(deadline(0, 0), Path::new("/x"), || {
            calls += 1;
            Attempt::Acquired(())
        })
        .expect("an uncontended attempt succeeds");
        assert_eq!(calls, 1);
    }

    /// A zero budget must still try once, or "do not wait" would silently mean
    /// "never attempt".
    #[test]
    fn a_zero_budget_attempts_once_and_then_reports_busy() {
        let mut calls = 0_u32;
        let error = wait_for(deadline(0, 0), Path::new("/x"), || {
            calls += 1;
            Attempt::<()>::Contended
        })
        .expect_err("permanent contention is busy");
        assert_eq!(calls, 1);
        assert_eq!(error.kind(), ErrorKind::LockBusy);
        assert_eq!(error.exit_code(), 75);
    }

    /// The loop is bounded by elapsed time rather than an attempt count, which
    /// is what an implementation that retried a fixed number of times would
    /// fail.
    #[test]
    fn contention_retries_until_the_budget_is_spent() {
        let mut calls = 0_u32;
        let error = wait_for(deadline(30, 1), Path::new("/x"), || {
            calls += 1;
            Attempt::<()>::Contended
        })
        .expect_err("permanent contention is busy");
        assert!(calls > 1, "a nonzero budget retries, got {calls} attempts");
        assert_eq!(error.kind(), ErrorKind::LockBusy);
    }

    /// A lock the platform refuses outright is an I/O fault, not contention:
    /// the two differ in whether retrying could ever help.
    #[test]
    fn a_refused_lock_is_reported_as_io_rather_than_busy() {
        let error = wait_for(deadline(1_000, 1), Path::new("/x"), || {
            Attempt::<()>::Failed(std::io::Error::from(std::io::ErrorKind::Unsupported))
        })
        .expect_err("a refused lock fails");
        assert_eq!(error.kind(), ErrorKind::Io);
    }

    #[test]
    fn one_mutex_is_interned_per_lock_path() {
        let first = scope_mutex(Path::new("/a/.credentials.lock"));
        let again = scope_mutex(Path::new("/a/.credentials.lock"));
        let other = scope_mutex(Path::new("/b/.credentials.lock"));
        assert!(std::ptr::eq(first, again));
        assert!(!std::ptr::eq(first, other));
    }
}
