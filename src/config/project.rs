//! Repository-bounded project configuration discovery.

use std::path::{Path, PathBuf};

/// Finds the first project file within the nearest repository.
///
/// Outside a repository there is no project layer at all, so a walk that
/// reaches the filesystem root without meeting a `.git` marker has not found a
/// project — it has left the concept behind. Returning a candidate there would
/// let a file in the home directory or at the root apply to every invocation
/// made from an unrelated tree.
pub(crate) fn discover(start: &Path) -> Option<PathBuf> {
    let mut candidate = None;
    let mut current = Some(start);
    while let Some(directory) = current {
        if candidate.is_none() {
            let file = directory.join(".claude-session-rs.toml");
            if file.is_file() {
                candidate = Some(file);
            }
        }
        if directory.join(".git").exists() {
            return candidate;
        }
        current = directory.parent();
    }
    None
}
