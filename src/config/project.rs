//! Repository-bounded project configuration discovery.

use std::path::{Path, PathBuf};

/// Finds the first project file while stopping at the nearest `.git` boundary.
pub(crate) fn discover(start: &Path) -> Option<PathBuf> {
    let mut current = Some(start);
    while let Some(directory) = current {
        let candidate = directory.join(".claude-session.toml");
        if candidate.is_file() {
            return Some(candidate);
        }
        if directory.join(".git").exists() {
            return None;
        }
        current = directory.parent();
    }
    None
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    #[test]
    fn project_discovery_stops_at_repository_boundary() {
        let tree = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            tree.path().join(".claude-session.toml"),
            "default_profile='outer'",
        )
        .expect("fixture");
        let repository = tree.path().join("repo");
        let child = repository.join("child");
        std::fs::create_dir_all(repository.join(".git")).expect("git marker");
        std::fs::create_dir_all(&child).expect("child");
        assert_eq!(discover(&child), None);
    }
}
