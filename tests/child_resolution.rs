#![allow(clippy::expect_used, clippy::pedantic, clippy::nursery)]

mod support;

use std::{fs, os::unix::fs::PermissionsExt};
use support::{Harness, make_executable};

#[test]
fn configured_child_is_terminal_when_missing() {
    let harness = Harness::new();
    let output = harness
        .command()
        .env("CLAUDE_SESSION_CHILD_BIN", harness.root().join("missing"))
        .output()
        .expect("wrapper");
    assert_eq!(output.status.code(), Some(127));
    assert!(String::from_utf8_lossy(&output.stderr).contains("ChildNotFound"));
}

#[test]
fn path_search_skips_empty_and_remembers_permission_denial() {
    let harness = Harness::new();
    let denied = harness.root().join("denied");
    fs::create_dir(&denied).expect("dir");
    fs::write(denied.join("claude"), b"x").expect("file");
    fs::set_permissions(denied.join("claude"), fs::Permissions::from_mode(0o644)).expect("mode");
    let good = harness.root().join("good");
    fs::create_dir(&good).expect("dir");
    fs::hard_link(harness.child(), good.join("claude")).expect("link");
    let path = std::env::join_paths([
        "",
        denied.to_str().expect("utf8"),
        good.to_str().expect("utf8"),
    ])
    .expect("PATH");
    let mut command = harness.command();
    command
        .env_remove("CLAUDE_SESSION_CHILD_BIN")
        .env("PATH", path);
    assert!(command.status().expect("wrapper").success());
}

/// The pre-flight check is advisory, so the exec's own errno decides the class.
/// A script whose interpreter is missing passes every check the ladder makes —
/// the file exists, is regular, and is executable — and still cannot run, which
/// is the only way to observe the exec's classification from outside.
///
/// `ENOEXEC` has no leg here because it cannot reach the wrapper: `execvp` hands
/// an unloadable file to `/bin/sh`, per `docs/reference/process-runtime.md#the-exec`.
#[test]
fn exec_failure_errno_is_classified() {
    let harness = Harness::new();
    let missing = harness.root().join("missing-interpreter");
    make_executable(&missing, b"#!/definitely/missing/interpreter\n");
    let output = harness
        .command()
        .env("CLAUDE_SESSION_CHILD_BIN", &missing)
        .output()
        .expect("wrapper");
    assert_eq!(output.status.code(), Some(127));
    assert!(String::from_utf8_lossy(&output.stderr).contains("error[ChildNotFound]"));
}

#[test]
fn reentry_marker_prevents_recursion() {
    let harness = Harness::new();
    let output = harness
        .command()
        .env("CLAUDE_SESSION_REENTRY", "1")
        .output()
        .expect("wrapper");
    assert_eq!(output.status.code(), Some(78));
    assert!(String::from_utf8_lossy(&output.stderr).contains("ChildRecursion"));
}

#[test]
fn hard_link_identity_prevents_recursion() {
    let harness = Harness::new();
    let binary = std::path::Path::new(env!("CARGO_BIN_EXE_claude-session"));
    let link_dir =
        tempfile::tempdir_in(binary.parent().expect("binary parent")).expect("link directory");
    let link = link_dir.path().join("wrapper-link");
    fs::hard_link(binary, &link).expect("hard link");
    let output = harness
        .command()
        .env("CLAUDE_SESSION_CHILD_BIN", link)
        .output()
        .expect("wrapper");
    assert_eq!(output.status.code(), Some(78));
}
