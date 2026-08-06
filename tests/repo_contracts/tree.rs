//! Walking the repository with the standard library alone.
//!
//! No walker crate is admitted: `walkdir` carries `Unlicense`, which the
//! licence allow-list does not name, and the walks these gates need are a
//! dozen lines of `read_dir`.

use std::fs;
use std::path::{Path, PathBuf};

/// Directories that are not repository sources. Excluding rather than
/// including is deliberate. An include list is opt-in, so a future top-level
/// directory of Markdown would be silently unscanned, and a gate that matches
/// zero rows reports success. An exclusion list fails noisily instead.
///
/// `target` is not hypothetical: `cargo package` writes a second copy of the
/// root README under it. `.draft` is the ignored workshop, excluded before the
/// first draft exists rather than after it first turns the gate red.
const EXCLUDED: &[&str] = &[
    ".direnv",
    ".draft",
    ".git",
    ".idea",
    ".vscode",
    "node_modules",
    "result",
    "target",
];

pub(crate) fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn excluded(name: &str) -> bool {
    EXCLUDED.contains(&name) || name.starts_with("mutants.out")
}

fn absolute(relative: &str) -> PathBuf {
    if relative.is_empty() {
        root().to_path_buf()
    } else {
        root().join(relative)
    }
}

fn join(base: &str, name: &str) -> String {
    if base.is_empty() {
        name.to_string()
    } else {
        format!("{base}/{name}")
    }
}

/// Immediate entries of `base`, as `(relative path, name)` pairs, sorted by
/// name. A symlink is reported as neither file nor directory by the callers
/// below, which is what keeps every walk inside the tree.
fn entries(base: &str) -> Vec<(String, String, fs::Metadata)> {
    let dir = absolute(base);
    let read = fs::read_dir(&dir)
        .unwrap_or_else(|err| panic!("{base:?} is required by the repository gates: {err}"));
    let mut out = Vec::new();
    for entry in read {
        let entry = entry.unwrap_or_else(|err| panic!("reading {base:?}: {err}"));
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = join(base, &name);
        let meta = fs::symlink_metadata(entry.path())
            .unwrap_or_else(|err| panic!("inspecting {path}: {err}"));
        out.push((path, name, meta));
    }
    out.sort_by(|a, b| a.1.cmp(&b.1));
    out
}

/// Immediate subdirectory names of `base`, sorted. A symlink to a directory is
/// not a directory here, matching `find -type d` without `-L`.
pub(crate) fn directories(base: &str) -> Vec<String> {
    entries(base)
        .into_iter()
        .filter(|(_, _, meta)| meta.is_dir())
        .map(|(_, name, _)| name)
        .collect()
}

/// Immediate file names of `base`, sorted.
pub(crate) fn names(base: &str) -> Vec<String> {
    entries(base)
        .into_iter()
        .filter(|(_, _, meta)| meta.is_file())
        .map(|(_, name, _)| name)
        .collect()
}

/// Every file at or below `base` whose name ends with one of `suffixes`,
/// relative to the repository root, sorted.
pub(crate) fn files(base: &str, suffixes: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    walk(base, &mut |path, name, meta| {
        if meta.is_file() && suffixes.iter().any(|suffix| name.ends_with(suffix)) {
            out.push(path.to_string());
        }
    });
    out.sort();
    out
}

/// Every file at or below `base` named exactly `name`.
pub(crate) fn files_named(base: &str, wanted: &str) -> Vec<String> {
    let mut out = Vec::new();
    walk(base, &mut |path, name, meta| {
        if meta.is_file() && name == wanted {
            out.push(path.to_string());
        }
    });
    out.sort();
    out
}

/// Every symlink at or below `base`. The plan-zone gate resolves links
/// lexically, which is equivalent to `realpath -m` only while this is empty.
pub(crate) fn symlinks(base: &str) -> Vec<String> {
    let mut out = Vec::new();
    walk(base, &mut |path, _, meta| {
        if meta.is_symlink() {
            out.push(path.to_string());
        }
    });
    out.sort();
    out
}

fn walk(base: &str, visit: &mut impl FnMut(&str, &str, &fs::Metadata)) {
    for (path, name, meta) in entries(base) {
        visit(&path, &name, &meta);
        if meta.is_dir() && !excluded(&name) {
            walk(&path, visit);
        }
    }
}

pub(crate) fn is_file(relative: &str) -> bool {
    absolute(relative).is_file()
}

/// Lexical `.` and `..` collapsing, without touching the filesystem.
///
/// `fs::canonicalize` is wrong twice over: it fails when the target does not
/// exist, which is the case the caller must report rather than crash on, and
/// it resolves symlinks. This matches `realpath -m` exactly while no component
/// of the path is a symlink, which `the_plan_zone_contains_no_symlink` and the
/// `check-symlinks` hook together keep true.
pub(crate) fn normalize(base: &str, target: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for part in base.split('/').chain(target.split('/')) {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.join("/")
}
