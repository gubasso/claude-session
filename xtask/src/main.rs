//! Development chores for `claude-session`.
//!
//! A thin CLI over the shipped crate's renderers. The rendering lives there so
//! package integration tests can prove freshness by linking the library rather
//! than shelling out to cargo; this member exists so the machinery never reaches
//! a user's install
//! ([ADR-0014](../../docs/decisions/ADR-0014-xtask-workspace-for-dev-tooling.md)).

use std::{path::Path, process::ExitCode};

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let (chore, flags) = arguments
        .split_first()
        .map_or(("", &[][..]), |(first, rest)| (first.as_str(), rest));
    match chore {
        "gen-config" => gen_config(flags.iter().any(|flag| flag == "--check")),
        other => {
            eprintln!("xtask: unknown chore {other:?}\n\nchores:\n  gen-config [--check]");
            ExitCode::from(64)
        }
    }
}

/// Rewrites the generated artifacts, or reports which are stale.
///
/// Staleness is a byte comparison against freshly rendered text, and a missing
/// file is stale. There is no hash file, no timestamp, and nothing to
/// invalidate: rendering is deterministic, so a commit that changes no field is
/// a natural no-op (`configuration.md#freshness`).
fn gen_config(check: bool) -> ExitCode {
    let root = repository_root();
    // Everything is rendered before anything is written, so a failure halfway
    // cannot leave the set half-regenerated.
    let rendered = claude_session::render::artifacts();
    let mut stale = Vec::new();
    for (relative, contents) in &rendered {
        let path = root.join(relative);
        let current = std::fs::read_to_string(&path).ok();
        if current.as_deref() != Some(contents.as_str()) {
            stale.push((relative, path, contents));
        }
    }

    if stale.is_empty() {
        if !check {
            println!("gen-config: 3 artifacts already fresh");
        }
        return ExitCode::SUCCESS;
    }

    if check {
        for (relative, _, _) in &stale {
            eprintln!("stale or missing: {relative}");
        }
        eprintln!("run `cargo xtask gen-config` to regenerate");
        return ExitCode::FAILURE;
    }

    let mut written = Vec::new();
    for (relative, path, contents) in &stale {
        if let Some(parent) = path.parent()
            && let Err(error) = std::fs::create_dir_all(parent)
        {
            eprintln!("xtask: cannot create {}: {error}", parent.display());
            return ExitCode::FAILURE;
        }
        if let Err(error) = std::fs::write(path, contents.as_bytes()) {
            eprintln!("xtask: cannot write {relative}: {error}");
            return ExitCode::FAILURE;
        }
        println!("regenerated: {relative}");
        written.push(relative.to_string());
    }

    // Generated files must be staged whole: a partially staged one commits
    // something the generator did not produce, and the gate cannot tell that
    // apart from a stale file (`configuration.md#freshness`).
    stage(&root, &written)
}

/// Stages exactly the paths that were rewritten.
fn stage(root: &Path, paths: &[String]) -> ExitCode {
    if paths.is_empty() {
        return ExitCode::SUCCESS;
    }
    match std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .arg("add")
        .args(paths)
        .status()
    {
        Ok(status) if status.success() => ExitCode::SUCCESS,
        Ok(status) => {
            eprintln!("xtask: git add exited {status}");
            ExitCode::FAILURE
        }
        Err(error) => {
            eprintln!("xtask: cannot run git: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Returns the workspace root, which is this member's parent.
///
/// From the manifest directory rather than the working directory, so the chore
/// behaves the same wherever it is invoked from.
fn repository_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf()
}
