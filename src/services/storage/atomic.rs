//! The lock-free half of the atomic-write sequence, steps 4 through 8.
//!
//! This module is for a write whose bytes are a pure function of the files it
//! read: composed settings, composition provenance, the last-used marker. It is
//! not for a lock scope, of which exactly one exists —
//! `accounts/<account>/.credentials.lock` — and which belongs with the account
//! subsystem that writes the pair it guards.
//!
//! Steps 1 through 3 and step 9 of `docs/reference/xdg-storage.md#the-sequence`
//! are that scope's and nothing else's. There is no scope to acquire here, so
//! there is nothing to release.

use std::{
    ffi::{OsStr, OsString},
    fs,
    io::Write as _,
    os::unix::{ffi::OsStrExt, fs::OpenOptionsExt as _},
    path::Path,
};

use crate::{
    adapters::filesystem::SystemFileSystem,
    error::{AppError, Diagnostic, ErrorKind},
};

/// Writes `bytes` to `path` by the lock-free atomic sequence, at `mode`.
pub(crate) fn write(path: &Path, bytes: &[u8], mode: u32) -> Result<(), AppError> {
    let (parent, name) = parts(path)?;
    let temporary = parent.join(temporary_name(name, std::process::id()));
    let handle = open_exclusive(&temporary, mode)?;
    let result = commit(&handle, &temporary, path, parent, bytes, mode);
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn commit(
    handle: &fs::File,
    temporary: &Path,
    path: &Path,
    parent: &Path,
    bytes: &[u8],
    mode: u32,
) -> Result<(), AppError> {
    let mut writer = handle;
    writer
        .write_all(bytes)
        .map_err(|error| io_error(temporary, &error))?;
    // The two syncs promise different things. Without this one the bytes are
    // not on the disk; without the directory sync below, a crash can resurrect
    // the old file or leave a zero-length one at the final name.
    handle
        .sync_all()
        .map_err(|error| io_error(temporary, &error))?;
    // Through the descriptor, so the final name is never briefly world-readable
    // and no path-based chmod can follow a link.
    SystemFileSystem::set_mode_at(handle, mode).map_err(|error| io_error(temporary, &error))?;
    fs::rename(temporary, path).map_err(|error| io_error(path, &error))?;
    let directory = fs::File::open(parent).map_err(|error| io_error(parent, &error))?;
    directory
        .sync_all()
        .map_err(|error| io_error(parent, &error))
}

/// Creates the temporary with `O_CREAT | O_EXCL`, clearing one orphan.
///
/// Two processes cannot share a process id, so a file already at this name is
/// an orphan by construction. It is removed and the create retried exactly
/// once; a second refusal is a real failure rather than a loop.
fn open_exclusive(temporary: &Path, mode: u32) -> Result<fs::File, AppError> {
    let create = || {
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(mode)
            .open(temporary)
    };
    match create() {
        Ok(handle) => Ok(handle),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            // Unlinking is using a wrapper-managed leaf, so the same
            // non-following checks run first. An orphan of this process's own
            // recycled id is a regular file it owns; anything else arrived by
            // the drift `docs/decisions/ADR-0061-protect-storage-from-accidental-local-drift.md`
            // scopes in, and is refused rather than quietly destroyed.
            if !is_own_regular_file(temporary) {
                return Err(foreign_temporary(temporary));
            }
            fs::remove_file(temporary).map_err(|error| io_error(temporary, &error))?;
            create().map_err(|error| io_error(temporary, &error))
        }
        Err(error) => Err(io_error(temporary, &error)),
    }
}

/// Reports whether a path is a non-link regular file owned by this user.
///
/// Read without following, so "is it a link" is answerable at all.
fn is_own_regular_file(path: &Path) -> bool {
    matches!(
        SystemFileSystem::look(path),
        Ok(Some(facts))
            if !facts.symlink
                && facts.regular
                && facts.uid == rustix::process::getuid().as_raw()
    )
}

fn foreign_temporary(path: &Path) -> AppError {
    AppError::new(
        ErrorKind::Permission,
        Diagnostic::new(
            "an atomic temporary is not a wrapper-owned regular file",
            path.display().to_string(),
            "storage-paths-owned: the name is this process's own temporary, but the \
            path is a link, a special file, or another user's",
            "Remove the path above, then run the command again.",
        ),
    )
}

fn parts(path: &Path) -> Result<(&Path, &OsStr), AppError> {
    match (path.parent(), path.file_name()) {
        (Some(parent), Some(name)) => Ok((parent, name)),
        _ => Err(AppError::new(
            ErrorKind::Internal,
            Diagnostic::new(
                "an atomic write has no destination",
                path.display().to_string(),
                "the path has no parent directory or no file name",
                "report this: an atomic write target is always a named file",
            ),
        )),
    }
}

/// Renders the atomic temporary name for a final name and a process id.
fn temporary_name(final_name: &OsStr, pid: u32) -> OsString {
    let mut name = OsString::from(".");
    name.push(final_name);
    name.push(format!(".{pid}.tmp"));
    name
}

/// Recovers the embedded process id from an atomic temporary name.
///
/// `None` for anything that is not `.<name>.<digits>.tmp`, and in particular
/// for `.credentials.lock`: a lock file must never be swept, and a grammar that
/// cannot match one is a surer guarantee than a name to skip.
pub(crate) fn temporary_pid(name: &OsStr) -> Option<u32> {
    let bytes = name.as_bytes();
    let rest = bytes.strip_prefix(b".")?.strip_suffix(b".tmp")?;
    let dot = rest.iter().rposition(|byte| *byte == b'.')?;
    let (stem, digits) = rest.split_at(dot);
    if stem.is_empty() {
        return None;
    }
    let digits = &digits[1..];
    if digits.is_empty() || !digits.iter().all(u8::is_ascii_digit) {
        return None;
    }
    std::str::from_utf8(digits).ok()?.parse().ok()
}

/// Removes orphaned atomic temporaries from one already-walked directory.
///
/// Returns nothing and fails nothing: this is a courtesy over a directory the
/// invocation already walked for its security checks, and a failed unlink is no
/// reason to fail a run. A temporary whose process id is live is left alone, so
/// a concurrent writer's rename can never be broken.
pub(crate) fn sweep(directory: &Path) {
    let Ok(entries) = SystemFileSystem::dir_entries(directory) else {
        return;
    };
    for name in entries {
        let Some(pid) = temporary_pid(&name) else {
            continue;
        };
        if SystemFileSystem::process_is_live(pid) {
            continue;
        }
        let path = directory.join(&name);
        // The name grammar alone does not make a path the wrapper's to unlink.
        // A link or a foreign artifact wearing a temporary's name is left where
        // it is: the sweep is a courtesy and skipping costs a stale file, while
        // removing costs someone else's.
        if !is_own_regular_file(&path) {
            continue;
        }
        if fs::remove_file(&path).is_ok() {
            tracing::debug!(
                op = "storage_sweep",
                path = %path.display(),
                status = "ok",
                "removed an orphaned atomic temporary"
            );
        }
    }
}

fn io_error(path: &Path, error: &std::io::Error) -> AppError {
    AppError::new(
        ErrorKind::Io,
        Diagnostic::new(
            "wrapper-managed storage could not be written",
            path.display().to_string(),
            error.to_string(),
            "check the path and the filesystem",
        ),
    )
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    fn os(value: &str) -> OsString {
        OsString::from(value)
    }

    #[test]
    fn a_temporary_name_round_trips_its_process_id() {
        let name = temporary_name(OsStr::new("profile-work-abc.json"), 4242);
        assert_eq!(name, os(".profile-work-abc.json.4242.tmp"));
        assert_eq!(temporary_pid(&name), Some(4242));
    }

    /// A final name carrying dots is the ordinary case here — the sidecar is
    /// `.compose.json` — so the process id is the last dotted field, not the
    /// second.
    #[test]
    fn a_dotted_final_name_still_yields_its_process_id() {
        let name = temporary_name(OsStr::new("profile-work-abc.compose.json"), 7);
        assert_eq!(temporary_pid(&name), Some(7));
    }

    /// The grammar cannot match a lock file. Unlinking one lets a holder
    /// destroy the file another is about to lock.
    #[test]
    fn a_lock_file_is_never_a_temporary() {
        assert_eq!(temporary_pid(&os(".credentials.lock")), None);
        assert_eq!(temporary_pid(&os(".accounts.lock")), None);
    }

    #[test]
    fn an_ordinary_or_malformed_name_is_not_a_temporary() {
        for value in [
            "profile-work-abc.json",
            ".x.notanumber.tmp",
            ".x..tmp",
            "..tmp",
            ".tmp",
            ".x.12.tmp.bak",
        ] {
            assert_eq!(temporary_pid(&os(value)), None, "{value} matched");
        }
    }
}
