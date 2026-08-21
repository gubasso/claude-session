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

/// Rule 1, at the two surfaces the ladder now decorates: colour is decoration,
/// so a stream that cannot render it gets the same characters without it, and
/// the machine document never carries one whatever the environment says.
#[test]
fn colour_decorates_the_diagnostic_without_changing_it() {
    let escape = |stream: &[u8]| stream.contains(&0x1b);
    let harness = Harness::new();
    let plain = harness.command().arg("--config").output().expect("wrapper");
    assert!(!escape(&plain.stderr), "a pipe carried an escape");
    let forced = harness
        .command()
        .arg("--config")
        .env("FORCE_COLOR", "1")
        .output()
        .expect("wrapper");
    assert!(escape(&forced.stderr), "FORCE_COLOR decorated nothing");
    assert_eq!(
        strip(&String::from_utf8_lossy(&forced.stderr)),
        String::from_utf8_lossy(&plain.stderr),
        "colour changed a character"
    );
    // Machine mode dominates every environment override.
    let machine = harness
        .command()
        .args(["account", "list", "--json"])
        .env("FORCE_COLOR", "1")
        .output()
        .expect("wrapper");
    assert!(!escape(&machine.stdout), "the document carried an escape");
    assert!(!escape(&machine.stderr));
}

/// Removes every SGR sequence, which is the only escape shape emitted.
fn strip(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find('\u{1b}') {
        out.push_str(&rest[..start]);
        let after = &rest[start..];
        let end = after.find('m').map_or(after.len(), |index| index + 1);
        rest = &after[end..];
    }
    out.push_str(rest);
    out
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
    assert!(harness.bound_command().status().expect("wrapper").success());
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
        .env("CLAUDE_SESSION_CHILD_BIN", harness.root().join("missing"))
        .output()
        .expect("wrapper");
    assert!(!output.status.success());
    let log = fs::read_to_string(
        harness
            .root()
            .join("state/claude-session/claude-session.log"),
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
    // A resolution failure renders the diagnostic, whatever the verbosity: the
    // mirror carries records, and a failure is not one of them.
    let stderr = |args: &[&str]| -> String {
        let harness = Harness::new();
        let output = harness
            .command()
            .args(args)
            .env("CLAUDE_SESSION_CHILD_BIN", harness.root().join("missing"))
            .output()
            .expect("wrapper");
        String::from_utf8_lossy(&output.stderr).into_owned()
    };
    let default = stderr(&[]);
    assert!(
        default.contains("error[ChildNotFound]"),
        "the default stream dropped the diagnostic:\n{default}"
    );
    let quiet = stderr(&["--quiet"]);
    assert!(
        !quiet.contains("claude-session: warn:") && !quiet.contains("claude-session: info:"),
        "--quiet mirrored below the error level:\n{quiet}"
    );
    assert!(
        quiet.contains("error[ChildNotFound]"),
        "--quiet dropped the diagnostic itself:\n{quiet}"
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
        verbose.contains("claude-session: info: this run will use the claude at"),
        "--verbose dropped info records:\n{verbose}"
    );
    // The mirror is prose; every field the record carries stays in the file.
    assert!(
        !verbose.contains("op=resolve_child"),
        "the mirror still carries machine fields:\n{verbose}"
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
        .env("CLAUDE_SESSION_CHILD_BIN", &unloadable)
        .output()
        .expect("wrapper");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("--- claude --version ---"),
        "the delimiter was not written:\n{stdout}"
    );
    assert!(
        stdout.contains("claude could not be run"),
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
    assert!(text.contains("Usage: claude-session version"), "{text}");
    assert!(!text.contains("--- claude"), "{text}");
    assert!(
        !harness.record_dir().join("argv").exists(),
        "requested help for version is answered by the wrapper alone"
    );
}

/// Rule 7, swept rather than argued: every human surface the wrapper writes is
/// checked for the shapes the rule retires, so a renderer added later cannot
/// quietly reintroduce one ([ADR-0093]).
///
/// [ADR-0093]: ../docs/decisions/ADR-0093-write-every-non-machine-surface-for-a-person.md
#[test]
fn no_human_surface_carries_a_machine_shape() {
    let harness = Harness::new();
    harness.initialize_token("work", b"sk-swept", b"sk-swept");
    harness.write_piece("work", "{}\n");
    harness.write_profile("work", "layers:\n  - work\n");
    harness
        .assert_command()
        .args(["account", "bind", "work", "--profile", "work"])
        .assert()
        .success();
    let surfaces = [
        vec!["account", "list"],
        vec!["--account", "work", "account", "status", "work"],
        vec!["--account", "work", "config"],
        vec!["--account", "work", "profile"],
        vec!["version"],
    ];
    for arguments in surfaces {
        let output = harness
            .command()
            .args(&arguments)
            .output()
            .expect("wrapper");
        let text = String::from_utf8_lossy(&output.stdout);
        for shape in [
            "status=",
            "usable:",
            "mode:",
            "recorded_at:",
            "selection_source:",
            "digest:",
            "fingerprint:",
            "age_seconds",
            "exit=",
            "](",
            "{",
            "}",
        ] {
            assert!(
                !text.contains(shape),
                "{arguments:?} wrote the machine shape {shape}:\n{text}"
            );
        }
        // Whatever it wrote, it wrote at the one wrap column. A line over it
        // holds a single unbreakable word, which the wrap deliberately lets
        // overflow so a path stays pasteable (`presentation.md`).
        for line in text.lines() {
            assert!(
                line.chars().count() <= 76 || !line.trim().contains(' '),
                "{arguments:?} exceeded the wrap column:\n{line}"
            );
        }
    }
    // A diagnostic is a human report too, and it is the one the sweep above
    // cannot see: it leaves by standard error and only when the verb fails. The
    // missing-session subject carried `account=..., profile=...` through the
    // rewrite because nothing here looked at this stream.
    let failures = [
        vec!["run"],
        vec!["--account", "work", "account", "status", "absent"],
        vec!["account", "bind", "work", "--profile", "absent"],
    ];
    for arguments in failures {
        let output = harness
            .command()
            .args(&arguments)
            .output()
            .expect("wrapper");
        assert!(
            !output.status.success(),
            "{arguments:?} was expected to fail:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let text = String::from_utf8_lossy(&output.stderr);
        for shape in ["account=", "profile=", "status=", "exit=", "](", "{", "}"] {
            assert!(
                !text.contains(shape),
                "{arguments:?} wrote the machine shape {shape}:\n{text}"
            );
        }
        assert!(
            text.contains("What to do:"),
            "{arguments:?} stated no next action:\n{text}"
        );
    }
}

/// The bargain the rewrite rests on: every identifier, counter, and code the
/// human form stopped printing is still in the machine document, so a caller
/// lost nothing by the change ([ADR-0093]).
///
/// [ADR-0093]: ../docs/decisions/ADR-0093-write-every-non-machine-surface-for-a-person.md
#[test]
fn every_machine_document_still_carries_what_the_human_form_dropped() {
    let harness = Harness::new();
    harness.initialize_token("work", b"sk-kept", b"sk-kept");
    harness.write_piece("work", "{}\n");
    harness.write_profile("work", "layers:\n  - work\n");
    harness
        .assert_command()
        .args(["account", "bind", "work", "--profile", "work"])
        .assert()
        .success();
    let document = |arguments: &[&str]| -> serde_json::Value {
        let output = harness.command().args(arguments).output().expect("wrapper");
        serde_json::from_slice(&output.stdout).expect("one document")
    };
    let list = document(&["--account", "work", "account", "list", "--json"]);
    assert_eq!(list["selection_source"], "flag");
    assert_eq!(list["accounts"][0]["mode"], "token");
    assert_eq!(list["accounts"][0]["usable"], true);
    assert_eq!(list["accounts"][0]["profile"], "work");
    let status = document(&["--account", "work", "account", "status", "work", "--json"]);
    for field in [
        "fingerprint",
        "recorded_at",
        "age_seconds",
        "profile_source",
    ] {
        assert!(status.get(field).is_some(), "{field} left the document");
    }
    let config = document(&["--account", "work", "config", "--json"]);
    assert!(config["profile"]["entry"]["digest"].is_string());
    assert_eq!(
        config["configuration"]["default_profile"]["source"],
        "account"
    );
    let doctor = document(&["--account", "work", "doctor", "--json"]);
    assert_eq!(doctor["schema_version"], 1);
    assert!(doctor["wrapper"]["checks"][0]["id"].is_string());
    assert!(doctor["wrapper"]["summary"]["total"].is_number());
}

/// Rule 1 across the report surfaces, not only the diagnostic: a pipe gets the
/// decorated bytes with the escapes taken out, and nothing else differs.
#[test]
fn colour_decorates_every_report_without_changing_it() {
    let harness = Harness::new();
    harness.initialize_token("work", b"sk-coloured", b"sk-coloured");
    harness.write_piece("work", "{}\n");
    harness.write_profile("work", "layers:\n  - work\n");
    harness
        .assert_command()
        .args(["account", "bind", "work", "--profile", "work"])
        .assert()
        .success();
    for arguments in [
        vec!["--account", "work", "account", "list"],
        vec!["--account", "work", "account", "status", "work"],
        vec!["--account", "work", "config"],
        vec!["--account", "work", "profile"],
    ] {
        let plain = harness
            .command()
            .args(&arguments)
            .output()
            .expect("wrapper");
        let forced = harness
            .command()
            .args(&arguments)
            .env("FORCE_COLOR", "1")
            .output()
            .expect("wrapper");
        assert!(
            !plain.stdout.contains(&0x1b),
            "{arguments:?} decorated a pipe"
        );
        assert!(
            forced.stdout.contains(&0x1b),
            "{arguments:?} decorated nothing under FORCE_COLOR"
        );
        assert_eq!(
            strip(&String::from_utf8_lossy(&forced.stdout)),
            String::from_utf8_lossy(&plain.stdout),
            "{arguments:?} changed a character"
        );
    }
}

/// The version report's published example says what the renderer says.
///
/// The account status example already had this test; the version report is the
/// other place a document publishes bytes a renderer produces, and it kept an
/// example from before the rewrite. The child path differs per machine, so the
/// sentences are compared and the path inside one is not.
#[test]
fn the_published_version_example_matches_the_renderer() {
    let harness = Harness::new();
    let output = harness.command().arg("version").output().expect("version");
    let rendered = support::flowed(&String::from_utf8_lossy(&output.stdout));
    let document = fs::read_to_string("docs/reference/cli-surface.md").expect("cli-surface.md");
    let example = document
        .split("```text\n  This is claude-session")
        .nth(1)
        .and_then(|rest| rest.split("```").next())
        .expect("the published example");
    for sentence in ["It wraps the claude at", "--- claude --version ---"] {
        assert!(
            support::flowed(example).contains(sentence),
            "the example dropped: {sentence}"
        );
        assert!(
            rendered.contains(sentence),
            "the renderer no longer writes: {sentence}\n{rendered}"
        );
    }
    assert!(
        rendered.contains("This is claude-session"),
        "the renderer no longer opens with the published sentence:\n{rendered}"
    );
}
