#![allow(clippy::expect_used, clippy::pedantic, clippy::nursery)]

mod support;

use std::{fs, os::unix::ffi::OsStringExt};
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
