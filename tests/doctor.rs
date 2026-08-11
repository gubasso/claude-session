#![allow(clippy::expect_used, clippy::pedantic, clippy::nursery)]

mod support;

use support::Harness;

const IDS: [&str; 13] = [
    "base-dirs-resolve",
    "runtime-dir-present",
    "wrapper-config-parses",
    "child-binary-resolves",
    "child-is-executable",
    "child-version-floor",
    "storage-paths-no-symlinks",
    "storage-paths-owned",
    "storage-paths-typed",
    "storage-directory-modes",
    "storage-secret-modes",
    "settings-compose",
    "settings-entry-consistent",
];

fn healthy(harness: &Harness) -> assert_cmd::Command {
    let mut command = harness.assert_command();
    command
        .env("XDG_RUNTIME_DIR", harness.root().join("runtime"))
        .env("CS_TEST_VERSION_STDOUT", "claude 2.1.220\n")
        .env("CS_TEST_DOCTOR_STDOUT", "native doctor\n");
    command
}

#[test]
fn doctor_human_report_preserves_catalog_order_and_text_shape() {
    let harness = Harness::new();
    let output = healthy(&harness).arg("doctor").output().expect("doctor");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).expect("text");
    let mut at = 0;
    for id in IDS {
        let found = text[at..].find(id).expect(id);
        at += found + id.len();
    }
    assert!(text.contains("Host\n[pass]"));
    assert!(text.contains("Session\n"));
    assert!(text.contains("wrapper status=pass total=13"));
    assert!(text.contains("\nchild status=pass exit=0\n"));
    assert!(text.contains("\ndoctor status=pass wrapper=0 child=0 exit=0\n"));
    assert!(text.ends_with("\n\n--- claude doctor ---\n\nnative doctor\n"));
}

#[test]
fn doctor_json_report_matches_the_public_catalog() {
    let harness = Harness::new();
    let output = healthy(&harness)
        .args(["doctor", "--json"])
        .output()
        .expect("doctor");
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(value["schema_version"], 1);
    let checks = value["wrapper"]["checks"].as_array().expect("checks");
    assert_eq!(checks.len(), 13);
    for (row, id) in checks.iter().zip(IDS) {
        assert_eq!(row["id"], id);
    }
    // Three levels, each stating its own status and code.
    assert_eq!(value["wrapper"]["summary"]["total"], 13);
    assert_eq!(value["wrapper"]["status"], "pass");
    assert_eq!(value["wrapper"]["summary"]["exit"], 0);
    assert_eq!(value["child"]["status"], "pass");
    assert_eq!(value["child"]["exit"], 0);
    assert_eq!(value["child"]["output"], "native doctor\n");
    assert_eq!(value["status"], "pass");
    assert_eq!(value["exit"], 0);
}

#[test]
fn doctor_list_human_runs_no_probes() {
    let harness = Harness::new();
    let output = harness
        .assert_command()
        .env_remove("HOME")
        .env("XDG_CONFIG_HOME", "relative")
        .env("CLAUDE_SESSION_CHILD_BIN", "/missing")
        .args(["doctor", "--list"])
        .output()
        .expect("list");
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let text = String::from_utf8(output.stdout).expect("text");
    assert_eq!(text.lines().count(), 13);
    assert!(!harness.record_dir().join("argv").exists());
}

#[test]
fn doctor_list_json_discovers_the_same_catalog() {
    let harness = Harness::new();
    let output = harness
        .assert_command()
        .args(["doctor", "--list", "--json"])
        .output()
        .expect("list");
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    let rows = value["checks"].as_array().expect("checks");
    assert_eq!(rows.len(), 13);
    for (row, id) in rows.iter().zip(IDS) {
        assert_eq!(row["id"], id);
        assert!(row.get("status").is_none());
    }
}

#[test]
fn doctor_strict_promotes_soft_warnings_only() {
    let harness = Harness::new();
    let mut normal = harness.assert_command();
    normal
        .env("CS_TEST_VERSION_STDOUT", "2.1.210\n")
        .arg("doctor")
        .assert()
        .success();
    let mut strict = harness.assert_command();
    strict
        .env("CS_TEST_VERSION_STDOUT", "2.1.210\n")
        .args(["doctor", "--strict"])
        .assert()
        .code(1);
}

#[test]
fn doctor_skips_inapplicable_session_checks_with_reasons() {
    let harness = Harness::new();
    let output = healthy(&harness)
        .args(["doctor", "--json"])
        .output()
        .expect("doctor");
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    let skipped = value["wrapper"]["checks"]
        .as_array()
        .expect("checks")
        .iter()
        .filter(|row| row["status"] == "skipped")
        .count();
    assert_eq!(skipped, 7);
}

#[test]
fn doctor_summary_counts_every_public_result_once() {
    doctor_json_report_matches_the_public_catalog();
}

#[test]
fn doctor_returns_the_first_hard_failure_in_catalog_order() {
    let harness = Harness::new();
    harness
        .assert_command()
        .env("CLAUDE_SESSION_CHILD_BIN", "/missing")
        .args(["doctor", "--strict"])
        .assert()
        .code(127);
}

#[test]
fn doctor_continues_after_a_subsystem_failure() {
    let harness = Harness::new();
    let output = harness
        .assert_command()
        .env("CLAUDE_SESSION_CHILD_BIN", "/missing")
        .args(["doctor", "--json"])
        .output()
        .expect("doctor");
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(
        value["wrapper"]["checks"]
            .as_array()
            .expect("checks")
            .last()
            .expect("last")["id"],
        "settings-entry-consistent"
    );
}

#[test]
fn doctor_warns_below_and_at_unparsable_child_versions() {
    for (version, code) in [
        ("2.1.210\n", 1),
        ("2.1.211\n", 0),
        ("2.2.0\n", 0),
        ("bad\n", 1),
    ] {
        let harness = Harness::new();
        harness
            .assert_command()
            .env("XDG_RUNTIME_DIR", harness.root().join("runtime"))
            .env("CS_TEST_VERSION_STDOUT", version)
            .args(["doctor", "--strict"])
            .assert()
            .code(code);
    }
}

#[test]
fn doctor_and_guard_emit_identical_remediation() {
    let harness = Harness::new();
    let state = harness.state();
    std::fs::create_dir_all(state.parent().expect("parent")).expect("state base");
    std::os::unix::fs::symlink(harness.root().join("elsewhere"), &state).expect("symlink");
    let guard = harness
        .assert_command()
        .args(["--account", "work"])
        .output()
        .expect("guard");
    let guard_text = String::from_utf8(guard.stderr).expect("diagnostic");
    let output = healthy(&harness)
        .args(["--account", "work", "doctor", "--json"])
        .output()
        .expect("doctor");
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    let hint = value["wrapper"]["checks"][6]["hint"]
        .as_str()
        .expect("hint");
    assert!(
        guard_text.contains(hint),
        "guard={guard_text}\ndoctor={hint}"
    );
}

#[test]
fn doctor_composes_child_output_and_carries_its_level() {
    // The child is the application being wrapped, so a failed child report is
    // a failed run here, with no `--strict` needed to make it bite.
    let harness = Harness::new();
    let mut command = healthy(&harness);
    command
        .env("CS_TEST_DOCTOR_STDERR", "native warning\n")
        .env("CS_TEST_DOCTOR_EXIT", "9")
        .arg("doctor");
    let output = command.output().expect("doctor");
    assert_eq!(output.status.code(), Some(69));
    let text = String::from_utf8(output.stdout.clone()).expect("text");
    assert!(text.contains("\nwrapper status=pass"));
    assert!(text.contains("\nchild status=fail exit=9\n"));
    assert!(text.contains("\ndoctor status=fail wrapper=0 child=9 exit=69\n"));
    assert!(output.stdout.ends_with(b"native doctor\n"));
    assert_eq!(output.stderr, b"native warning\n");

    let output = healthy(&harness)
        .env("CS_TEST_DOCTOR_EXIT", "9")
        .args(["doctor", "--json"])
        .output()
        .expect("doctor");
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(value["wrapper"]["status"], "pass");
    assert_eq!(value["child"]["status"], "fail");
    assert_eq!(value["child"]["exit"], 9);
    assert_eq!(value["status"], "fail");
    assert_eq!(value["exit"], 69);
}

#[test]
fn a_wrapper_hard_failure_outranks_the_child_code() {
    let harness = Harness::new();
    let output = harness
        .assert_command()
        .env("CLAUDE_SESSION_CHILD_BIN", "/missing")
        .env("CS_TEST_DOCTOR_EXIT", "9")
        .arg("doctor")
        .output()
        .expect("doctor");
    assert_eq!(output.status.code(), Some(127));
}

#[test]
fn doctor_reports_bootstrap_failures_in_the_requested_mode() {
    let harness = Harness::new();
    let output = harness
        .assert_command()
        .env_remove("HOME")
        .env_remove("XDG_CONFIG_HOME")
        .env_remove("XDG_STATE_HOME")
        .env_remove("XDG_DATA_HOME")
        .env_remove("XDG_CACHE_HOME")
        .args(["doctor", "--json"])
        .output()
        .expect("doctor");
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(value["wrapper"]["checks"][0]["status"], "fail");
    assert_eq!(value["wrapper"]["checks"][12]["status"], "skipped");
    assert_eq!(value["child"]["status"], "skipped");
    assert_eq!(output.status.code(), Some(69));

    let config = harness.config_base().join("config.toml");
    std::fs::create_dir_all(config.parent().expect("parent")).expect("config dir");
    std::fs::write(&config, "unknown = true\n").expect("bad config");
    let output = healthy(&harness)
        .args(["doctor", "--json"])
        .output()
        .expect("doctor");
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(value["wrapper"]["checks"][2]["status"], "fail");
    assert_eq!(
        value["wrapper"]["checks"].as_array().expect("checks").len(),
        13
    );
}

#[test]
fn doctor_flags_are_verb_scoped_and_singleton() {
    let harness = Harness::new();
    harness
        .assert_command()
        .args(["doctor", "--unknown"])
        .assert()
        .code(64);
    harness
        .assert_command()
        .args(["doctor", "--json", "--json"])
        .assert()
        .code(64);
}
