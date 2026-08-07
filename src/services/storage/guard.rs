//! Validating and preparing wrapper-managed paths immediately before use.
//!
//! This module is for running the five storage checks over the components of a
//! managed path. It is not for deciding what a path means, which is
//! `domain::paths`, and not for writing bytes, which is `super::atomic`.
//!
//! The policy boundary is exact: a wrapper-managed component begins at the
//! `claude-session` namespace directory inside an XDG base. `$HOME`,
//! `.config`, and `.local/state` are the operating system's or the user's, and
//! are never read, never checked, and never corrected.

use std::path::{Path, PathBuf};

use crate::{
    adapters::filesystem::{PathFacts, SystemFileSystem},
    domain::checks::StorageCheck,
    error::{AppError, Diagnostic, ErrorKind},
};

/// What a wrapper-managed path is expected to be.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Expected {
    /// A directory, at mode `0700`.
    Directory,
    /// A wrapper-owned private file, at mode `0600`.
    PrivateFile,
}

impl Expected {
    /// Returns the artifact table's name for this type.
    const fn spelling(self) -> &'static str {
        match self {
            Self::Directory => "directory",
            Self::PrivateFile => "regular file",
        }
    }
    /// Returns the mode the artifact table assigns this type.
    const fn mode(self) -> u32 {
        match self {
            Self::Directory => 0o700,
            Self::PrivateFile => 0o600,
        }
    }
}

/// Validates every managed component of `path` below `root`.
///
/// The walk stops at the first component that does not exist: a path being
/// created is validated as far as it is real, and the rest is this run's to
/// make. No result is held: `docs/reference/xdg-storage.md#how-a-path-is-validated`
/// requires the check to run immediately before each operation, against the
/// state that operation will meet, so caching one would be caching the answer
/// to a question about a moment that has passed.
pub(crate) fn validate(root: &Path, path: &Path, expected: Expected) -> Result<(), AppError> {
    walk(root, path, expected, false)
}

/// Validates and idempotently creates every managed directory component.
pub(crate) fn ensure_directory(root: &Path, path: &Path) -> Result<(), AppError> {
    walk(root, path, Expected::Directory, true)
}

fn walk(root: &Path, path: &Path, expected: Expected, create: bool) -> Result<(), AppError> {
    let components = managed_components(root, path)?;
    let last = components.len().saturating_sub(1);
    let mut current = root.to_path_buf();
    for (index, component) in components.into_iter().enumerate() {
        if index > 0 {
            current.push(component);
        }
        let leaf = index == last;
        let want = if leaf { expected } else { Expected::Directory };
        match SystemFileSystem::look(&current).map_err(|error| io_error(&current, &error))? {
            None => {
                if !create {
                    return Ok(());
                }
                SystemFileSystem::create_dir_private(&current)
                    .map_err(|error| io_error(&current, &error))?;
            }
            Some(facts) => inspect(&current, facts, want)?,
        }
    }
    Ok(())
}

/// Runs the five checks over one existing component.
///
/// Order matters only in that a link is refused before anything reads through
/// it. The type and ownership checks are independent, and the mode check is a
/// correction rather than a refusal, so it runs last.
fn inspect(path: &Path, facts: PathFacts, expected: Expected) -> Result<(), AppError> {
    if facts.symlink {
        return Err(refuse(
            StorageCheck::NoSymlinks,
            path,
            expected.spelling(),
            "symbolic link",
            None,
        ));
    }
    if facts.uid != current_uid() {
        return Err(refuse(
            StorageCheck::Owned,
            path,
            expected.spelling(),
            "path owned by another user",
            None,
        ));
    }
    if !matches_type(facts, expected) {
        return Err(refuse(
            StorageCheck::Typed,
            path,
            expected.spelling(),
            actual_type(facts),
            None,
        ));
    }
    correct_mode(path, facts, expected)
}

/// Corrects drift toward the artifact table's mode, or refuses if it cannot.
///
/// A correction is a pass, not an unhealthy state — `doctor.md#results-and-exit`
/// makes that explicit — so it warns and proceeds. Both mode rows of
/// `xdg-storage.md#filesystem-security` read "correct, then proceed"; they
/// differ only in mechanism. A directory is corrected by path, after its own
/// non-following check. A file is corrected through a descriptor this walk
/// opens, because a path-based `chmod(2)` dereferences a symbolic link.
fn correct_mode(path: &Path, facts: PathFacts, expected: Expected) -> Result<(), AppError> {
    if facts.mode == expected.mode() {
        return Ok(());
    }
    let (check, outcome) = match expected {
        Expected::Directory => (
            StorageCheck::DirectoryModes,
            SystemFileSystem::set_dir_mode(path),
        ),
        // The open revalidates type and ownership from the handle and corrects
        // the mode through it, so nothing swapped since the metadata pass is
        // what gets chmod'd.
        Expected::PrivateFile => (
            StorageCheck::SecretModes,
            SystemFileSystem::open_private_file(path).map(drop),
        ),
    };
    match outcome {
        Ok(()) => {
            tracing::warn!(
                op = "storage_guard",
                check = check.id(),
                path = %path.display(),
                was = format!("{:04o}", facts.mode),
                now = format!("{:04o}", expected.mode()),
                status = "pass",
                "restricted a wrapper-managed path"
            );
            Ok(())
        }
        Err(_) => Err(refuse(
            check,
            path,
            expected.spelling(),
            expected.spelling(),
            Some(expected.mode()),
        )),
    }
}

fn refuse(
    check: StorageCheck,
    path: &Path,
    expected: &str,
    actual: &str,
    mode: Option<u32>,
) -> AppError {
    tracing::error!(
        op = "storage_guard",
        check = check.id(),
        path = %path.display(),
        status = "fail",
        "err.kind" = check.kind().spelling(),
        "refused a wrapper-managed path"
    );
    AppError::new(check.kind(), check.diagnostic(path, expected, actual, mode))
}

const fn matches_type(facts: PathFacts, expected: Expected) -> bool {
    match expected {
        Expected::Directory => facts.directory,
        Expected::PrivateFile => facts.regular,
    }
}

const fn actual_type(facts: PathFacts) -> &'static str {
    if facts.directory {
        "directory"
    } else if facts.regular {
        "regular file"
    } else {
        "special file"
    }
}

fn current_uid() -> u32 {
    rustix::process::getuid().as_raw()
}

fn io_error(path: &Path, error: &std::io::Error) -> AppError {
    AppError::new(
        ErrorKind::Io,
        Diagnostic::new(
            "wrapper-managed storage could not be read",
            path.display().to_string(),
            error.to_string(),
            "check the path and the filesystem",
        ),
    )
}

/// Splits `path` into the components the wrapper owns, starting at `root`.
///
/// The returned list always begins with `root` itself: the namespace directory
/// is at the boundary rather than above it, so it is checked. A path outside
/// `root` is a programming error rather than an invitation to walk the whole
/// tree, which is the failure mode a permissive `strip_prefix` would hide.
fn managed_components(root: &Path, path: &Path) -> Result<Vec<PathBuf>, AppError> {
    let Ok(relative) = path.strip_prefix(root) else {
        return Err(AppError::new(
            ErrorKind::Internal,
            Diagnostic::new(
                "a storage path escaped its namespace",
                path.display().to_string(),
                format!("{} is not below {}", path.display(), root.display()),
                "report this: a managed path must be built from the XDG namespace",
            ),
        ));
    };
    let mut components = vec![root.to_path_buf()];
    components.extend(
        relative
            .components()
            .map(|part| PathBuf::from(part.as_os_str())),
    );
    Ok(components)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    fn facts(directory: bool, regular: bool, symlink: bool, mode: u32) -> PathFacts {
        PathFacts {
            symlink,
            directory,
            regular,
            uid: current_uid(),
            mode,
        }
    }

    #[test]
    fn the_namespace_directory_is_the_first_managed_component() {
        let root = Path::new("/s/claude-session");
        let parts =
            managed_components(root, Path::new("/s/claude-session/accounts/work")).expect("below");
        assert_eq!(
            parts,
            vec![
                PathBuf::from("/s/claude-session"),
                PathBuf::from("accounts"),
                PathBuf::from("work"),
            ]
        );
    }

    #[test]
    fn the_namespace_directory_alone_is_one_component() {
        let root = Path::new("/s/claude-session");
        assert_eq!(
            managed_components(root, root).expect("below"),
            vec![PathBuf::from("/s/claude-session")]
        );
    }

    /// A path outside the namespace must fail loudly. Walking it would apply
    /// the wrapper's policy to the user's own tree.
    #[test]
    fn a_path_outside_the_namespace_is_an_internal_error() {
        let error = managed_components(Path::new("/s/claude-session"), Path::new("/etc/passwd"))
            .expect_err("outside");
        assert_eq!(error.kind(), ErrorKind::Internal);
    }

    #[test]
    fn a_symbolic_link_is_refused_before_its_type_is_considered() {
        let error = inspect(
            Path::new("/s/claude-session/accounts"),
            facts(true, false, true, 0o700),
            Expected::Directory,
        )
        .expect_err("a link is refused");
        assert_eq!(error.kind(), ErrorKind::Permission);
        assert!(error.diagnostic().why.contains("storage-paths-no-symlinks"));
    }

    /// The ownership leg has no hermetic end-to-end test: a non-root process
    /// cannot create a path owned by another user. This covers the predicate
    /// the integration suite reaches only through the symlink leg of the same
    /// acceptance sentence.
    #[test]
    fn a_foreign_owner_is_refused() {
        let mut foreign = facts(true, false, false, 0o700);
        foreign.uid = current_uid().wrapping_add(1);
        let error = inspect(
            Path::new("/s/claude-session/accounts"),
            foreign,
            Expected::Directory,
        )
        .expect_err("a foreign owner is refused");
        assert_eq!(error.kind(), ErrorKind::Permission);
        assert!(error.diagnostic().why.contains("storage-paths-owned"));
    }

    #[test]
    fn a_wrong_type_names_both_types() {
        let error = inspect(
            Path::new("/s/claude-session/composed"),
            facts(false, true, false, 0o600),
            Expected::Directory,
        )
        .expect_err("a file where a directory belongs");
        assert!(error.diagnostic().why.contains("storage-paths-typed"));
        assert!(error.diagnostic().hint.contains("is a regular file"));
        assert!(error.diagnostic().hint.contains("must be a directory"));
    }

    /// The owner page's `storage-secret-modes` row is "correct, then proceed",
    /// the same verdict the directory row carries. The correction goes through
    /// a descriptor rather than the path, so a link cannot be dereferenced.
    #[test]
    fn a_private_file_at_the_wrong_mode_is_corrected_through_a_descriptor() {
        use std::os::unix::fs::PermissionsExt as _;

        let directory = tempfile::tempdir().expect("tempdir");
        let path = directory.path().join("profile-work-abc.json");
        std::fs::write(&path, b"{}").expect("fixture");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).expect("drift");

        let observed = SystemFileSystem::look(&path)
            .expect("look")
            .expect("present");
        inspect(&path, observed, Expected::PrivateFile)
            .expect("a drifted private file is corrected");
        assert_eq!(
            SystemFileSystem::look(&path)
                .expect("look")
                .expect("present")
                .mode,
            0o600
        );
    }

    /// A leaf the walk cannot open is refused rather than passed: the mode row
    /// is a correction, and a correction that did not happen is a failure.
    #[test]
    fn a_private_file_that_cannot_be_corrected_is_refused() {
        let error = inspect(
            Path::new("/s/claude-session/composed/profile-work-abc.json"),
            facts(false, true, false, 0o644),
            Expected::PrivateFile,
        )
        .expect_err("an unopenable private file");
        assert!(error.diagnostic().why.contains("storage-secret-modes"));
    }

    #[test]
    fn a_correct_component_passes_every_check() {
        assert!(
            inspect(
                Path::new("/s/claude-session/accounts"),
                facts(true, false, false, 0o700),
                Expected::Directory,
            )
            .is_ok()
        );
    }
}
