//! The profile listing verb, observed through the binary.
//!
//! Discovery reads a user-owned configuration directory, so every behaviour
//! here belongs in the integration lane by
//! `docs/reference/testing-and-quality.md`.

#![allow(clippy::expect_used, clippy::pedantic, clippy::nursery)]

mod support;

use support::Harness;

const PROFILE: &str = "layers:\n  - base\n";

/// Writes three profiles out of alphabetical order, so ordering is observed
/// rather than inherited from the directory's own iteration order.
fn fixtures(harness: &Harness) {
    harness.write_piece("base", r#"{"model":"sonnet"}"#);
    harness.write_profile("zeta", PROFILE);
    harness.write_profile("personal", PROFILE);
    harness.write_profile("work", PROFILE);
}

fn stdout_of(output: &std::process::Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("utf-8 stdout")
}

#[test]
fn profile_lists_the_available_names_in_order() {
    let harness = Harness::new();
    fixtures(&harness);
    let output = harness
        .command()
        .arg("profile")
        .output()
        .expect("profile runs");
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stdout_of(&output), "personal\nwork\nzeta\n");
}

/// A user who has written no profile has asked a question whose answer is
/// "none", which is a result rather than a diagnostic
/// (`exit-codes.md#resolving-a-profile-name`).
#[test]
fn profile_reports_an_empty_list_without_failing() {
    let harness = Harness::new();
    let output = harness
        .command()
        .arg("profile")
        .output()
        .expect("profile runs");
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stdout.is_empty(), "stdout: {:?}", output.stdout);
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
}

#[test]
fn profile_marks_the_selected_name() {
    let harness = Harness::new();
    fixtures(&harness);
    let output = harness
        .command()
        .args(["--profile", "work", "profile"])
        .output()
        .expect("profile runs");
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stdout_of(&output), "personal\nwork (selected)\nzeta\n");
}

#[test]
fn profile_json_reports_the_same_names_as_the_human_form() {
    let harness = Harness::new();
    fixtures(&harness);
    let human = harness
        .command()
        .args(["--profile", "work", "profile"])
        .output()
        .expect("profile runs");
    let json = harness
        .command()
        .args(["--profile", "work", "profile", "--json"])
        .output()
        .expect("profile runs");
    assert_eq!(json.status.code(), Some(0));
    let document: serde_json::Value = serde_json::from_slice(&json.stdout).expect("one document");
    assert!(
        document.get("schema_version").is_none(),
        "schema_version appears on doctor alone"
    );
    let profiles = document["profiles"].as_array().expect("profiles array");
    let names: Vec<&str> = profiles
        .iter()
        .map(|value| value["name"].as_str().expect("name"))
        .collect();
    let human_names: Vec<String> = stdout_of(&human)
        .lines()
        .map(|line| line.trim_end_matches(" (selected)").to_owned())
        .collect();
    assert_eq!(names, human_names);
    assert_eq!(profiles[1]["selected"], serde_json::Value::Bool(true));
    assert!(
        profiles[0].get("selected").is_none(),
        "an absent optional field is omitted, never false"
    );
    assert!(
        profiles[0]["path"]
            .as_str()
            .expect("path")
            .ends_with("/profiles/personal.yaml")
    );
}

#[test]
fn profile_writes_data_to_stdout_and_nothing_to_stderr() {
    let harness = Harness::new();
    fixtures(&harness);
    let output = harness
        .command()
        .arg("profile")
        .output()
        .expect("profile runs");
    assert!(!output.stdout.is_empty());
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
}

/// A stray file in a user-owned directory is not a wrapper failure, so it is
/// skipped rather than reported.
#[test]
fn profile_skips_a_file_that_is_not_a_profile_name() {
    let harness = Harness::new();
    fixtures(&harness);
    harness.write_profile("Not-An-Id", PROFILE);
    std::fs::write(
        harness.config_base().join("profiles").join("notes.txt"),
        "x",
    )
    .expect("stray file");
    std::fs::create_dir_all(harness.config_base().join("profiles").join("nested.yaml"))
        .expect("stray directory");
    let output = harness
        .command()
        .arg("profile")
        .output()
        .expect("profile runs");
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stdout_of(&output), "personal\nwork\nzeta\n");
}

#[test]
fn profile_help_is_a_result_on_standard_output() {
    let harness = Harness::new();
    let flag = harness
        .command()
        .args(["profile", "--help"])
        .output()
        .expect("profile --help runs");
    let verb = harness
        .command()
        .args(["help", "profile"])
        .output()
        .expect("help profile runs");
    assert_eq!(flag.status.code(), Some(0));
    assert_eq!(verb.status.code(), Some(0));
    assert!(!flag.stdout.is_empty());
    assert_eq!(flag.stdout, verb.stdout);
}

#[test]
fn profile_never_reaches_the_child() {
    let harness = Harness::new();
    fixtures(&harness);
    harness
        .command()
        .arg("profile")
        .output()
        .expect("profile runs");
    assert!(
        !harness.record_dir().join("argv").exists(),
        "the profile verb is answered by the wrapper"
    );
}
