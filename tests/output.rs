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

/// The ladder is decided but not yet applied: slice 013 carries it to bytes.
/// Until then the absence has to be provable rather than assumed, and the
/// strongest case is the one the ladder resolves to on: an active
/// `FORCE_COLOR`, which no lower rung can overrule.
#[test]
fn no_surface_emits_an_escape_byte_yet() {
    let escape = |stream: &[u8]| stream.contains(&0x1b);
    let harness = Harness::new();
    // A usage failure renders the human diagnostic on standard error.
    let human = harness
        .command()
        .arg("--config")
        .env("FORCE_COLOR", "1")
        .output()
        .expect("wrapper");
    assert!(!escape(&human.stderr), "the diagnostic carried an escape");
    assert!(!escape(&human.stdout));
    // The composed delimiter is the only standard-output surface today.
    let harness = Harness::new();
    let composed = harness
        .command()
        .arg("--version")
        .env("FORCE_COLOR", "1")
        .env("CS_TEST_STDOUT", "native")
        .output()
        .expect("wrapper");
    assert!(!escape(&composed.stdout), "the delimiter carried an escape");
    assert!(!escape(&composed.stderr));
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
    assert!(text.contains("Usage: claude-session-rs"));
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
        .env(
            "CLAUDE_SESSION_RS_CHILD_BIN",
            harness.root().join("missing"),
        )
        .output()
        .expect("wrapper");
    assert!(output.stdout.is_empty());
    assert!(!output.stderr.is_empty());
}

#[test]
fn log_mode_rotation_and_record_shape() {
    let harness = Harness::new();
    let state = harness.root().join("state/claude-session-rs");
    fs::create_dir_all(&state).expect("state fixture");
    fs::write(
        state.join("claude-session-rs.log"),
        vec![b'x'; 8 * 1024 * 1024],
    )
    .expect("large log");
    assert!(harness.bound_command().status().expect("wrapper").success());
    let log_path = state.join("claude-session-rs.log");
    let metadata = fs::metadata(&log_path).expect("active log");
    assert_eq!(metadata.permissions().mode() & 0o777, 0o600);
    assert!(state.join("claude-session-rs.log.1").is_file());
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

/// The log file exists for a user who ran with no flags, so the record that
/// names the failure has to survive the flush ordering ([ADR-0080]). Asserting
/// its absence on a successful run cannot catch a boundary that reports after
/// joining the sink.
///
/// [ADR-0080]: ../docs/decisions/ADR-0080-order-the-boundary-as-report-flush-exit.md
#[test]
fn a_failing_invocation_records_its_error_in_the_log() {
    let harness = Harness::new();
    let output = harness
        .command()
        .env(
            "CLAUDE_SESSION_RS_CHILD_BIN",
            harness.root().join("missing"),
        )
        .output()
        .expect("wrapper");
    assert!(!output.status.success());
    let log = fs::read_to_string(
        harness
            .root()
            .join("state/claude-session-rs/claude-session-rs.log"),
    )
    .expect("UTF-8 structured log");
    assert!(
        log.contains("err.kind=ChildNotFound"),
        "the failure was reported but never logged:\n{log}"
    );
    // The log and the diagnostic must spell the kind the same way, or a reader
    // learns two vocabularies for one failure.
    let stderr = String::from_utf8(output.stderr).expect("UTF-8 diagnostic");
    assert!(stderr.contains("ChildNotFound"), "{stderr}");
}

/// The verbosity ladder governs the stderr mirror and nothing else. Neither
/// end of it had coverage, and both ends were unimplemented: the default
/// mirrored nothing, and `--quiet` was parsed and then ignored.
#[test]
fn the_verbosity_ladder_governs_the_diagnostic_mirror() {
    // A resolution failure emits a warning-level record before the error.
    let stderr = |args: &[&str]| -> String {
        let harness = Harness::new();
        let output = harness
            .command()
            .args(args)
            .env(
                "CLAUDE_SESSION_RS_CHILD_BIN",
                harness.root().join("missing"),
            )
            .output()
            .expect("wrapper");
        String::from_utf8_lossy(&output.stderr).into_owned()
    };
    let default = stderr(&[]);
    assert!(
        default.contains("level=error"),
        "the default mirror dropped an error record:\n{default}"
    );
    let quiet = stderr(&["--quiet"]);
    assert!(
        !quiet.contains("level=warn") && !quiet.contains("level=info"),
        "--quiet mirrored below the error level:\n{quiet}"
    );
    // The one info record the wrapper emits names a resolution that succeeded,
    // so this case needs the real child rather than the missing one above.
    let harness = Harness::new();
    let output = harness
        .command()
        .arg("--verbose")
        .output()
        .expect("wrapper");
    let verbose = String::from_utf8_lossy(&output.stderr);
    assert!(
        verbose.contains("level=info") && verbose.contains("op=resolve_child"),
        "--verbose dropped info records:\n{verbose}"
    );
}

/// A child that resolves but cannot be executed reaches the spawn, not the
/// resolver. The composed section is replaced either way, and its absence does
/// not change the exit status.
#[test]
fn a_spawn_failure_names_its_condition_in_place_of_the_section() {
    let harness = Harness::new();
    // Executable bits set, but not a loadable image: `execve` returns ENOEXEC
    // after every resolution check has already passed.
    let unloadable = harness.root().join("unloadable");
    support::make_executable(&unloadable, b"\x7fELF not really\n");
    let output = harness
        .command()
        .arg("--version")
        .env("CLAUDE_SESSION_RS_CHILD_BIN", &unloadable)
        .output()
        .expect("wrapper");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("--- claude --version ---"),
        "the delimiter was not written:\n{stdout}"
    );
    assert!(
        stdout.contains("claude unavailable:"),
        "a spawn failure left the section silently empty:\n{stdout}"
    );
    assert_eq!(output.status.code(), Some(0));
}

/// `version` is wrapper-only as a verb — the child owns `--version`, not a
/// `version` subcommand — so its requested help composes nothing
/// (`docs/reference/cli-surface.md#help`).
#[test]
fn requested_version_help_is_a_result_and_composes_nothing() {
    let harness = Harness::new();
    let flag = harness
        .command()
        .args(["version", "--help"])
        .output()
        .expect("version --help runs");
    let verb = harness
        .command()
        .args(["help", "version"])
        .output()
        .expect("help version runs");
    assert_eq!(flag.status.code(), Some(0));
    assert_eq!(verb.status.code(), Some(0));
    assert!(!flag.stdout.is_empty());
    assert_eq!(flag.stdout, verb.stdout);
    let text = String::from_utf8(flag.stdout).expect("utf-8 help");
    assert!(text.contains("Usage: claude-session-rs version"), "{text}");
    assert!(!text.contains("--- claude"), "{text}");
    assert!(
        !harness.record_dir().join("argv").exists(),
        "requested help for version is answered by the wrapper alone"
    );
}
