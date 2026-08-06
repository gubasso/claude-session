#![allow(clippy::expect_used, clippy::pedantic, clippy::nursery)]

mod support;

use std::{fs, os::unix::fs::PermissionsExt};
use support::{Harness, os};

#[test]
fn diagnostic_stream_ownership() {
    let harness = Harness::new();
    let mut command = harness.assert_command();
    let assertion = command.arg("--config").assert().code(64);
    let output = assertion.get_output();
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("error[Usage]"));
}

#[test]
fn json_error_shape() {
    let harness = Harness::new();
    let config = harness.root().join("bad.toml");
    fs::write(&config, "unknown_key = true\n").expect("invalid config");
    let output = harness
        .command()
        .args([
            "--config".into(),
            config.into_os_string(),
            "version".into(),
            "--json".into(),
        ])
        .output()
        .expect("wrapper");
    assert!(output.stdout.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&output.stderr).expect("JSON error");
    assert_eq!(value["kind"], "Config");
    for key in ["what", "where", "why", "hint"] {
        assert!(value.get(key).is_some());
    }
    assert!(value.get("child_exit").is_none());
}

#[test]
fn color_precedence() {
    let harness = Harness::new();
    let output = harness
        .command()
        .arg("--config")
        .env("NO_COLOR", "")
        .env("FORCE_COLOR", "1")
        .output()
        .expect("wrapper");
    assert!(!output.stderr.contains(&0x1b));
}

#[test]
fn exact_delimiter_bytes() {
    let harness = Harness::new();
    let output = harness
        .command()
        .arg("--version")
        .env("CS_TEST_STDOUT", "native")
        .output()
        .expect("wrapper");
    assert!(
        output
            .stdout
            .windows(b"\n\n--- claude --version ---\n\n".len())
            .any(|window| window == b"\n\n--- claude --version ---\n\n")
    );
}

#[test]
fn help_output_matches_snapshot() {
    let harness = Harness::new();
    let output = harness
        .command()
        .arg("--help")
        .env("CS_TEST_STDOUT", "native-help")
        .output()
        .expect("wrapper");
    let text = String::from_utf8_lossy(&output.stdout);
    insta::assert_snapshot!("\n\n--- claude --help ---\n\n", @r"

--- claude --help ---

");
    assert!(text.contains("Usage: claude-session"));
    assert!(text.contains("--- claude --help ---"));
    assert!(text.ends_with("native-help"));
}

#[test]
fn claimed_read_only_surfaces_compose_child_bytes() {
    for flag in ["--help", "--version"] {
        let harness = Harness::new();
        let native = os(vec![b'n', 0x80, b'x']);
        let output = harness
            .command()
            .arg(flag)
            .env("CS_TEST_STDOUT", native)
            .output()
            .expect("wrapper");
        let delimiter = format!("\n\n--- claude {flag} ---\n\n").into_bytes();
        let position = output
            .stdout
            .windows(delimiter.len())
            .position(|window| window == delimiter)
            .expect("delimiter");
        assert_eq!(
            &output.stdout[position + delimiter.len()..],
            &[b'n', 0x80, b'x']
        );
    }
}

#[test]
fn version_json_reports_resolution_states() {
    let harness = Harness::new();
    let output = harness
        .command()
        .args(["version", "--json"])
        .env("CS_TEST_STDOUT", "claude 2.1.220\n")
        .output()
        .expect("wrapper");
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("JSON");
    assert_eq!(value["child"]["status"], "ok");
    let harness = Harness::new();
    let output = harness
        .command()
        .args(["version", "--json"])
        .env("CS_TEST_STDOUT", os(vec![0x80]))
        .output()
        .expect("wrapper");
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("JSON");
    assert_eq!(value["child"]["status"], "unparsable");
}

#[test]
fn passthrough_errors_never_pollute_stdout() {
    let harness = Harness::new();
    let output = harness
        .command()
        .env("CLAUDE_SESSION_CHILD_BIN", harness.root().join("missing"))
        .output()
        .expect("wrapper");
    assert!(output.stdout.is_empty());
    assert!(!output.stderr.is_empty());
}

#[test]
fn log_mode_rotation_and_record_shape() {
    let harness = Harness::new();
    let state = harness.root().join("state/claude-session");
    fs::create_dir_all(&state).expect("state fixture");
    fs::write(
        state.join("claude-session.log"),
        vec![b'x'; 8 * 1024 * 1024],
    )
    .expect("large log");
    assert!(harness.command().status().expect("wrapper").success());
    let log_path = state.join("claude-session.log");
    let metadata = fs::metadata(&log_path).expect("active log");
    assert_eq!(metadata.permissions().mode() & 0o777, 0o600);
    assert!(state.join("claude-session.log.1").is_file());
    let log = fs::read_to_string(log_path).expect("UTF-8 structured log");
    for field in [
        "ts=",
        "level=",
        "target=",
        "op=resolve_child",
        "msg=",
        "status=ok",
    ] {
        assert!(log.contains(field), "missing log field {field}");
    }
    assert!(!log.contains("dur_ms="));
    assert!(!log.contains("err.kind="));
}
