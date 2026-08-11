#![allow(clippy::expect_used, clippy::pedantic, clippy::nursery)]

mod support;

use std::{fs, os::unix::ffi::OsStringExt, os::unix::fs::PermissionsExt};
use support::{Harness, read_nul};

#[test]
fn configuration_precedence_and_provenance() {
    let harness = Harness::new();
    let user_child = harness.root().join("user-child");
    fs::hard_link(harness.child(), &user_child).expect("user child");
    let env_child = harness.root().join("env-child");
    fs::hard_link(harness.child(), &env_child).expect("env child");
    let config = harness.root().join("config.toml");
    fs::write(&config, format!("child_bin = {:?}\n", user_child)).expect("config");
    let status = harness
        .command()
        .args(["--config".into(), config.into_os_string()])
        .env("CLAUDE_SESSION_CHILD_BIN", &env_child)
        .status()
        .expect("wrapper");
    assert!(status.success());
    assert_eq!(
        read_nul(&harness.record_dir().join("argv"))[0],
        env_child.as_os_str().as_encoded_bytes()
    );
}

#[test]
fn unknown_configuration_key_names_key_and_file() {
    let harness = Harness::new();
    let config = harness.root().join("bad.toml");
    fs::write(&config, "child_bni = '/tmp/x'\n").expect("config");
    let output = harness
        .command()
        .args(["--config".into(), config.clone().into_os_string()])
        .output()
        .expect("wrapper");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(78));
    assert!(stderr.contains("child_bni"));
    assert!(stderr.contains(config.to_str().expect("utf8")));
}

#[test]
fn project_discovery_stops_at_repository_boundary() {
    let harness = Harness::new();
    let repo = harness.root().join("repo");
    let child = repo.join("nested");
    fs::create_dir_all(repo.join(".git")).expect("marker");
    fs::create_dir_all(&child).expect("cwd");
    fs::write(
        harness.root().join(".claude-session.toml"),
        "child_bin='/not/allowed'\n",
    )
    .expect("outer");
    let mut command = harness.command();
    command.current_dir(child);
    assert!(command.status().expect("wrapper").success());
}

#[test]
fn non_utf8_config_path_is_accepted() {
    let harness = Harness::new();
    let path = harness
        .root()
        .join(std::ffi::OsString::from_vec(vec![b'c', 0x80]));
    fs::write(&path, format!("child_bin = {:?}\n", harness.child())).expect("config");
    assert!(
        harness
            .command()
            .args(["--config".into(), path.into_os_string()])
            .status()
            .expect("wrapper")
            .success()
    );
}

/// The project layer had only negative coverage, so nothing proved a file
/// inside a repository is ever found. A discovery that silently matched
/// nothing would have passed every test that existed.
///
/// The refusal of a forbidden key is the detector: only a file that was found
/// and read can be rejected for what it contains.
#[test]
fn a_project_file_inside_a_repository_is_read() {
    let harness = Harness::new();
    let repo = harness.root().join("repo");
    let nested = repo.join("nested");
    fs::create_dir_all(repo.join(".git")).expect("marker");
    fs::create_dir_all(&nested).expect("cwd");
    fs::write(repo.join(".claude-session.toml"), "child_bin='/anything'\n").expect("project file");
    let mut command = harness.command();
    command.current_dir(&nested);
    let output = command.output().expect("wrapper");
    assert_eq!(output.status.code(), Some(78));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not allowed in a project file"),
        "the project file was never read:\n{stderr}"
    );
}

#[test]
fn a_project_file_cannot_select_an_account() {
    let harness = Harness::new();
    let repo = harness.root().join("repo");
    fs::create_dir_all(repo.join(".git")).expect("marker");
    fs::write(
        repo.join(".claude-session.toml"),
        "default_account='work'\n",
    )
    .expect("project file");
    let mut command = harness.command();
    command.current_dir(&repo);
    let output = command.output().expect("wrapper");
    assert_eq!(output.status.code(), Some(78));
    assert!(String::from_utf8_lossy(&output.stderr).contains("default_account"));
}

/// Outside a repository there is no project layer at all. Without a ceiling the
/// walk reaches the filesystem root, so a file in the home directory applies to
/// every invocation made from an unrelated tree.
#[test]
fn no_project_layer_applies_outside_a_repository() {
    let harness = Harness::new();
    let nested = harness.root().join("loose/nested");
    fs::create_dir_all(&nested).expect("cwd");
    fs::write(
        harness.root().join("loose/.claude-session.toml"),
        "child_bin='/not/allowed'\n",
    )
    .expect("stray file");
    let mut command = harness.command();
    command.current_dir(&nested);
    assert!(
        command.status().expect("wrapper").success(),
        "a project file outside any repository was read"
    );
}

/// A file the user explicitly named has three distinct failures with three
/// distinct fixes. Collapsing them into one code tells the reader to correct a
/// value when the real problem is that the file cannot be opened at all.
#[test]
fn an_unreadable_named_config_is_typed() {
    let harness = Harness::new();
    let path = harness.root().join("unreadable.toml");
    fs::write(&path, "default_profile='x'\n").expect("config");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).expect("mode");
    let output = harness
        .command()
        .args(["--config".into(), path.clone().into_os_string()])
        .output()
        .expect("wrapper");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(77),
        "an unreadable file reported {:?}:\n{stderr}",
        output.status.code()
    );
    assert!(stderr.contains("Permission"), "{stderr}");
    assert!(
        stderr.contains(path.to_str().expect("utf8")),
        "the diagnostic did not name the file:\n{stderr}"
    );
}

/// An identifier becomes a path component in the storage tree, so its grammar
/// is a usage contract every layer shares. Before this, a bad name in a file
/// exited `Config` and the same name on the command line exited `Usage` — two
/// codes for one mistake, disagreeing with the two pages that own the rule.
#[test]
fn an_invalid_identifier_from_any_layer_exits_usage() {
    let harness = Harness::new();
    let config = harness.root().join("bad-profile.toml");
    fs::write(&config, "default_profile='Work'\n").expect("config");
    let output = harness
        .command()
        .args(["--config".into(), config.clone().into_os_string()])
        .output()
        .expect("wrapper");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(64), "user file layer:\n{stderr}");
    assert!(stderr.contains("Work"), "{stderr}");
    assert!(
        stderr.contains(config.to_str().expect("utf8")),
        "the diagnostic did not name the file:\n{stderr}"
    );

    let output = harness
        .command()
        .env("CLAUDE_SESSION_DEFAULT_ACCOUNT", "has.dot")
        .output()
        .expect("wrapper");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(64),
        "environment layer:\n{stderr}"
    );
    assert!(stderr.contains("has.dot"), "{stderr}");
    assert!(
        stderr.contains("CLAUDE_SESSION_DEFAULT_ACCOUNT"),
        "the diagnostic did not name the variable:\n{stderr}"
    );

    let output = harness
        .command()
        .args(["--profile", "has/slash"])
        .output()
        .expect("wrapper");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(64), "command line:\n{stderr}");
    assert!(stderr.contains("has/slash"), "{stderr}");
}
