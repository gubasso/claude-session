//! Repository-bounded project configuration discovery.

use std::{
    ffi::OsStr,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
};

/// Finds the first project file within the nearest repository.
///
/// Outside a repository there is no project layer at all, so a walk that
/// reaches the filesystem root without meeting a `.git` marker has not found a
/// project — it has left the concept behind. Returning a candidate there would
/// let a file in the home directory or at the root apply to every invocation
/// made from an unrelated tree.
///
/// A ceiling ends the walk the way it ends git's own discovery: the walk never
/// enters a listed directory from below, so a `.git` marker at or above a
/// ceiling does not make the working directory part of a repository
/// (ADR-0130). A ceiling equal to the starting directory does not stop it,
/// because git applies a ceiling to ancestors alone.
pub(crate) fn discover(start: &Path, ceilings: &[PathBuf]) -> Option<PathBuf> {
    let mut candidate = None;
    let mut current = Some(start);
    while let Some(directory) = current {
        if candidate.is_none() {
            let file = directory.join(".claude-session.toml");
            if file.is_file() {
                candidate = Some(file);
            }
        }
        if directory.join(".git").exists() {
            return candidate;
        }
        current = directory.parent().filter(|parent| {
            !ceilings
                .iter()
                .any(|ceiling| ceiling_bytes(ceiling) == directory_bytes(parent))
        });
    }
    None
}

/// A ceiling in the form git compares it: its bytes with one trailing
/// separator removed, and the root as the empty string. Git removes exactly
/// one, so `/repo/docs//` is no ceiling for `/repo/docs`.
fn ceiling_bytes(ceiling: &Path) -> &[u8] {
    let bytes = ceiling.as_os_str().as_bytes();
    if bytes == b"/" {
        return &[];
    }
    bytes.strip_suffix(b"/").unwrap_or(bytes)
}

/// A directory of the walk in the form a ceiling is compared with. The walk
/// starts from the resolved working directory, so only the root carries a
/// trailing separator.
fn directory_bytes(directory: &Path) -> &[u8] {
    let bytes = directory.as_os_str().as_bytes();
    if bytes == b"/" { &[] } else { bytes }
}

/// Reads `GIT_CEILING_DIRECTORIES` as git reads it.
///
/// The value is a colon-separated list. An entry that is empty or relative is
/// skipped. An entry before the first empty one is resolved through its
/// symbolic links, and dropped when it cannot be resolved, as git does, so it
/// matches a working directory that the operating system reports in resolved
/// form. An entry after an empty one is kept as written and compared as a
/// string, so `/repo/docs/.` does not match `/repo/docs`.
pub(crate) fn ceilings(value: Option<&OsStr>) -> Vec<PathBuf> {
    let Some(value) = value else {
        return Vec::new();
    };
    let mut resolve = true;
    let mut found = Vec::new();
    for entry in std::env::split_paths(value) {
        if entry.as_os_str().is_empty() {
            resolve = false;
            continue;
        }
        if !entry.is_absolute() {
            continue;
        }
        if !resolve {
            found.push(entry);
        } else if let Ok(resolved) = std::fs::canonicalize(&entry) {
            found.push(resolved);
        }
    }
    found
}
