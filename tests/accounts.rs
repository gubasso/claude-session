#![allow(clippy::expect_used, clippy::pedantic, clippy::nursery)]

mod support;

use std::{fs, os::unix::fs::PermissionsExt as _};
use support::{Harness, bytes, mode, read_invocations, read_nul};

fn list_json(harness: &Harness) -> serde_json::Value {
    let output = harness
        .command()
        .args(["account", "list", "--json"])
        .output()
        .expect("list");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("json")
}

#[test]
fn account_list_discovers_directories_without_a_registry() {
    let harness = Harness::new();
    harness.initialize_login("work");
    harness.initialize_login("alpha");
    let value = list_json(&harness);
    assert_eq!(value["accounts"][0]["name"], "alpha");
    assert_eq!(value["accounts"][1]["name"], "work");
    assert!(!harness.state().join("accounts.json").exists());
}

#[test]
fn account_list_empty_is_a_successful_complete_report() {
    let harness = Harness::new();
    let value = list_json(&harness);
    assert_eq!(value["selection_source"], "none");
    assert_eq!(value["accounts"].as_array().expect("accounts").len(), 0);
    assert!(!harness.record_dir().join("argv").exists());
}

/// Acceptance line 2 names three inputs and one prohibition, so every leg is
/// here: directory security, mode metadata, the stored artifact's presence,
/// and no child invocation behind any of them.
#[test]
fn account_list_reports_local_usability_without_spawning_a_child() {
    let harness = Harness::new();
    let accounts = harness.state().join("accounts");

    // Healthy: all three inputs answer yes.
    harness.initialize_login("healthy");

    // Directory security: the child configuration directory is a link out of
    // the private tree. `lstat` on the credential path would follow it, so
    // only a check on `config` itself can catch this.
    harness.initialize_login("linked");
    let elsewhere = harness.root().join("elsewhere");
    fs::create_dir_all(&elsewhere).expect("fixture");
    fs::write(elsewhere.join(".credentials.json"), b"outside").expect("fixture");
    fs::remove_file(accounts.join("linked/config/.credentials.json")).expect("fixture");
    fs::remove_dir(accounts.join("linked/config")).expect("fixture");
    std::os::unix::fs::symlink(&elsewhere, accounts.join("linked/config")).expect("fixture");

    // Artifact presence: valid metadata, no child-owned saved login.
    harness.initialize_login("artifactless");
    fs::remove_file(accounts.join("artifactless/config/.credentials.json")).expect("fixture");

    // Mode metadata: present but unparsable.
    harness.initialize_login("malformed");
    fs::write(accounts.join("malformed/auth-mode.json"), b"not-json").expect("fixture");

    let value = list_json(&harness);
    let reported: Vec<(&str, &str, bool)> = value["accounts"]
        .as_array()
        .expect("accounts")
        .iter()
        .map(|row| {
            (
                row["name"].as_str().expect("name"),
                row["mode"].as_str().expect("mode"),
                row["usable"].as_bool().expect("usable"),
            )
        })
        .collect();
    assert_eq!(
        reported,
        vec![
            ("artifactless", "login", false),
            ("healthy", "login", true),
            ("linked", "login", false),
            ("malformed", "invalid", false),
        ]
    );
    assert!(!harness.record_dir().join("argv").exists());
    assert_eq!(
        fs::read(elsewhere.join(".credentials.json")).expect("fixture"),
        b"outside",
        "the report must not touch a file outside the account tree"
    );
}

#[test]
fn account_list_marks_insecure_missing_and_malformed_accounts_unusable() {
    let harness = Harness::new();
    let accounts = harness.state().join("accounts");
    fs::create_dir_all(accounts.join("broken")).expect("fixture");
    fs::set_permissions(accounts.join("broken"), fs::Permissions::from_mode(0o700)).expect("mode");
    fs::write(accounts.join("broken/auth-mode.json"), b"not-json").expect("metadata");
    fs::set_permissions(
        accounts.join("broken/auth-mode.json"),
        fs::Permissions::from_mode(0o600),
    )
    .expect("mode");
    let value = list_json(&harness);
    assert_eq!(value["accounts"][0]["mode"], "invalid");
    assert_eq!(value["accounts"][0]["usable"], false);
}

#[test]
fn account_list_token_probe_never_reads_token_bytes() {
    let harness = Harness::new();
    let account = harness.state().join("accounts/token");
    fs::create_dir_all(&account).expect("fixture");
    fs::set_permissions(&account, fs::Permissions::from_mode(0o700)).expect("mode");
    fs::write(
        account.join("auth-mode.json"),
        concat!(
            "{\"mode\":\"token\",\"recorded_at\":\"2026-08-11T00:00:00Z\",",
            "\"fingerprint\":\"deadbeef\"}\n"
        ),
    )
    .expect("metadata");
    fs::set_permissions(
        account.join("auth-mode.json"),
        fs::Permissions::from_mode(0o600),
    )
    .expect("mode");
    fs::write(account.join("oauth-token"), b"do-not-read").expect("token");
    fs::set_permissions(
        account.join("oauth-token"),
        fs::Permissions::from_mode(0o000),
    )
    .expect("token mode");
    let value = list_json(&harness);
    assert_eq!(value["accounts"][0]["mode"], "token");
    assert_eq!(value["accounts"][0]["usable"], true);
    fs::set_permissions(
        account.join("oauth-token"),
        fs::Permissions::from_mode(0o600),
    )
    .expect("restore fixture mode");
    assert_eq!(
        fs::read(account.join("oauth-token")).expect("fixture read"),
        b"do-not-read"
    );
}

#[test]
fn account_list_reports_selection_source_precedence() {
    let harness = Harness::new();
    harness.initialize_login("work");
    let marker = harness.state().join("state/last-account");
    fs::create_dir_all(marker.parent().expect("parent")).expect("marker parent");
    fs::write(&marker, b"work").expect("marker");
    fs::set_permissions(&marker, fs::Permissions::from_mode(0o600)).expect("marker mode");
    assert_eq!(list_json(&harness)["selection_source"], "marker");
    let output = harness
        .command()
        .args(["--account", "work", "account", "list", "--json"])
        .output()
        .expect("list");
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(value["selection_source"], "flag");
    assert_eq!(value["accounts"][0]["selected"], true);
}

#[test]
fn account_list_human_and_json_carry_the_same_fields() {
    let harness = Harness::new();
    harness.initialize_login("work");
    let human = harness
        .command()
        .args(["--account", "work", "account", "list"])
        .output()
        .expect("human list");
    let text = String::from_utf8(human.stdout).expect("text");
    for field in [
        "selection_source: flag",
        "account: work",
        "mode: login",
        "usable: true",
        "selected: true",
    ] {
        assert!(text.contains(field), "{field} missing from {text}");
    }
    let json = harness
        .command()
        .args(["--account", "work", "account", "list", "--json"])
        .output()
        .expect("JSON list");
    let value: serde_json::Value = serde_json::from_slice(&json.stdout).expect("json");
    assert_eq!(value["selection_source"], "flag");
    assert_eq!(value["accounts"][0]["name"], "work");
    assert_eq!(value["accounts"][0]["mode"], "login");
    assert_eq!(value["accounts"][0]["usable"], true);
    assert_eq!(value["accounts"][0]["selected"], true);
}

#[test]
fn native_login_requires_a_controlling_terminal_before_side_effects() {
    let harness = Harness::new();
    let output = harness
        .detached_command(&["account", "login", "work"])
        .output()
        .expect("detached login");
    assert_eq!(
        output.status.code(),
        Some(69),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!harness.state().join("accounts/work").exists());
}

#[test]
fn native_login_delegates_to_the_child_owned_shared_config() {
    let harness = Harness::new();
    let output = harness
        .terminal_command("account login work")
        .env("CS_TEST_CREATE_CREDENTIAL", "1")
        .output()
        .expect("terminal login");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let account = harness.state().join("accounts/work");
    assert_eq!(mode(&account), 0o700);
    assert_eq!(mode(&account.join("config")), 0o700);
    assert_eq!(mode(&account.join("auth-mode.json")), 0o600);
    assert_eq!(
        fs::read(account.join("config/.credentials.json")).expect("credential"),
        b"child-owned-login-bytes"
    );
    let metadata: serde_json::Value =
        serde_json::from_slice(&fs::read(account.join("auth-mode.json")).expect("metadata"))
            .expect("json");
    assert_eq!(metadata["mode"], "login");
    assert!(
        metadata["recorded_at"]
            .as_str()
            .expect("timestamp")
            .ends_with('Z')
    );
    assert_eq!(
        &read_nul(&harness.record_dir().join("argv"))[1..],
        &[b"auth".to_vec(), b"login".to_vec()]
    );
}

#[test]
fn failed_first_native_login_removes_only_the_incomplete_account() {
    let harness = Harness::new();
    let output = harness
        .terminal_command("account login work")
        .env("CS_TEST_EXIT", "9")
        .output()
        .expect("terminal login");
    assert_eq!(output.status.code(), Some(77));
    assert!(!harness.state().join("accounts/work").exists());
}

#[test]
fn failed_repeat_native_login_preserves_the_existing_account() {
    let harness = Harness::new();
    harness.initialize_login("work");
    let before = fs::read(harness.state().join("accounts/work/auth-mode.json")).expect("metadata");
    let output = harness
        .terminal_command("account login work")
        .env("CS_TEST_EXIT", "9")
        .output()
        .expect("terminal login");
    assert_eq!(output.status.code(), Some(77));
    assert_eq!(
        fs::read(harness.state().join("accounts/work/auth-mode.json")).expect("metadata"),
        before
    );
    assert_eq!(
        fs::read(
            harness
                .state()
                .join("accounts/work/config/.credentials.json")
        )
        .expect("credential"),
        b"child-owned-fixture"
    );
}

#[test]
fn native_login_json_keeps_child_bytes_off_stdout_and_reports_child_exit() {
    let harness = Harness::new();
    let stdout = harness.root().join("login-success.json");
    let output = harness
        .terminal_command(&format!("account login work --json > {}", stdout.display()))
        .env("CS_TEST_CREATE_CREDENTIAL", "1")
        .env("CS_TEST_STDOUT", "child-login-bytes")
        .output()
        .expect("terminal login");
    assert!(output.status.success());
    let value: serde_json::Value =
        serde_json::from_slice(&fs::read(&stdout).expect("wrapper stdout")).expect("json");
    assert_eq!(value["account"], "work");
    assert!(
        !fs::read(&stdout)
            .expect("wrapper stdout")
            .windows(b"child-login-bytes".len())
            .any(|window| window == b"child-login-bytes")
    );

    let harness = Harness::new();
    let stdout = harness.root().join("login-failure.json");
    let output = harness
        .terminal_command(&format!("account login work --json > {}", stdout.display()))
        .env("CS_TEST_STDOUT", "child-failure-bytes")
        .env("CS_TEST_EXIT", "9")
        .output()
        .expect("terminal login");
    assert_eq!(output.status.code(), Some(77));
    assert!(fs::read(&stdout).expect("wrapper stdout").is_empty());
    let terminal = String::from_utf8_lossy(&output.stdout);
    assert!(terminal.contains("child-failure-bytes"));
    assert!(terminal.contains("\"child_exit\":9"));
}

#[test]
fn marker_is_written_before_the_selected_account_exec() {
    let harness = Harness::new();
    harness.initialize_login("work");
    let marker = harness.state().join("state/last-account");
    let status = harness
        .companion_profile_command()
        .args(["--account", "work", "run"])
        .env("CS_TEST_MARKER_PATH", &marker)
        .status()
        .expect("wrapper");
    assert!(status.success());
    assert_eq!(fs::read(&marker).expect("marker"), b"work");
    assert_eq!(mode(&marker), 0o600);
    let visibility =
        fs::read_to_string(harness.record_dir().join("marker-visible")).expect("visibility");
    assert_eq!(visibility.lines().collect::<Vec<_>>(), ["0", "1"]);
    // The same acceptance sentence binds the launch environment: the account's
    // configuration directory is set, and no wrapper token variable is.
    let environment = read_nul(&harness.record_dir().join("environ"));
    let config = harness.state().join("accounts/work/config");
    assert!(
        environment
            .windows(2)
            .any(|pair| pair[0] == b"CLAUDE_CONFIG_DIR" && pair[1] == bytes(config.as_os_str()))
    );
    assert!(
        !environment
            .iter()
            .any(|key| key == b"CLAUDE_CODE_OAUTH_TOKEN"),
        "a login-mode launch sets no wrapper token variable"
    );
}

#[test]
fn failed_selected_launch_does_not_advance_the_marker() {
    let harness = Harness::new();
    harness.initialize_login("work");
    let marker = harness.state().join("state/last-account");
    let output = harness
        .companion_profile_command()
        .args(["--account", "work"])
        .env("CS_TEST_VERSION_STDOUT", "bad\n")
        .output()
        .expect("wrapper");
    assert_eq!(output.status.code(), Some(69));
    assert!(!marker.exists());
}

#[test]
fn marker_selection_is_used_when_no_higher_rung_answers() {
    let harness = Harness::new();
    harness.initialize_login("work");
    let marker = harness.state().join("state/last-account");
    fs::create_dir_all(marker.parent().expect("parent")).expect("parent");
    fs::write(&marker, b"work").expect("marker");
    fs::set_permissions(&marker, fs::Permissions::from_mode(0o600)).expect("mode");
    assert!(
        harness
            .companion_profile_command()
            .arg("run")
            .status()
            .expect("wrapper")
            .success()
    );
    let environment = read_nul(&harness.record_dir().join("environ"));
    let config = harness.state().join("accounts/work/config");
    assert!(
        environment
            .windows(2)
            .any(|pair| pair[0] == b"CLAUDE_CONFIG_DIR" && pair[1] == bytes(config.as_os_str()))
    );
}

#[test]
fn login_launch_below_the_version_floor_refuses_before_exec() {
    let harness = Harness::new();
    harness.initialize_login("work");
    let output = harness
        .companion_profile_command()
        .args(["--account", "work", "run"])
        .env("CS_TEST_VERSION_STDOUT", "2.1.210 (Claude Code)\n")
        .output()
        .expect("wrapper");
    assert_eq!(output.status.code(), Some(69));
    assert!(String::from_utf8_lossy(&output.stderr).contains(concat!(
        "Upgrade claude to 2.1.211 or newer before using a saved-login account. ",
        "Token accounts are unaffected and still work below that version."
    )));
    assert_eq!(
        &read_nul(&harness.record_dir().join("argv"))[1..],
        &[b"--version".to_vec()]
    );

    // The same acceptance sentence covers the version that does not parse, and
    // it owes the same catalog remediation rather than a second wording.
    let harness = Harness::new();
    harness.initialize_login("work");
    let output = harness
        .companion_profile_command()
        .args(["--account", "work", "run"])
        .env("CS_TEST_VERSION_STDOUT", "garbage\n")
        .output()
        .expect("wrapper");
    assert_eq!(output.status.code(), Some(69));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("claude's version could not be read"),
        "{stderr}"
    );
    assert!(
        stderr.contains(concat!(
            "Upgrade claude to 2.1.211 or newer before using a saved-login account. ",
            "Token accounts are unaffected and still work below that version."
        )),
        "{stderr}"
    );
    assert_eq!(
        read_invocations(&harness.record_dir().join("invocations")),
        vec![vec![b"--version".to_vec()]],
        "the refusal precedes the exec"
    );
}

/// `err.kind` is public machine-readable API and picks the remediation, so a
/// storage defect on the selected account keeps its own kind instead of being
/// folded into the semantic `Auth` the missing-metadata case owns.
#[test]
fn selected_launch_reports_the_kind_its_owner_assigns() {
    let harness = Harness::new();
    harness.initialize_login("work");
    let metadata = harness.state().join("accounts/work/auth-mode.json");
    fs::remove_file(&metadata).expect("fixture");
    let output = harness
        .companion_profile_command()
        .args(["--account", "work", "run"])
        .output()
        .expect("wrapper");
    assert_eq!(output.status.code(), Some(77));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("error[Auth]"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let harness = Harness::new();
    harness.initialize_login("work");
    let metadata = harness.state().join("accounts/work/auth-mode.json");
    fs::remove_file(&metadata).expect("fixture");
    std::os::unix::fs::symlink(harness.root().join("outside.json"), &metadata).expect("fixture");
    let output = harness
        .companion_profile_command()
        .args(["--account", "work", "run"])
        .output()
        .expect("wrapper");
    assert_eq!(output.status.code(), Some(77));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("error[Permission]"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn login_launch_with_unparsable_version_refuses_before_exec() {
    let harness = Harness::new();
    harness.initialize_login("work");
    let status = harness
        .companion_profile_command()
        .args(["--account", "work", "run"])
        .env("CS_TEST_VERSION_STDOUT", "garbage\n")
        .status()
        .expect("wrapper");
    assert_eq!(status.code(), Some(69));
    assert!(!harness.state().join("state/last-account").exists());
}

#[test]
fn login_launch_at_the_floor_writes_marker_then_execs() {
    let harness = Harness::new();
    harness.initialize_login("work");
    assert!(
        harness
            .companion_profile_command()
            .args(["--account", "work", "run"])
            .env("CS_TEST_VERSION_STDOUT", "2.1.211 (Claude Code)\n")
            .status()
            .expect("wrapper")
            .success()
    );
    let marker = harness.state().join("state/last-account");
    assert_eq!(fs::read(&marker).expect("marker"), b"work");
    let invocations = read_invocations(&harness.record_dir().join("invocations"));
    assert_eq!(invocations[0], vec![b"--version".to_vec()]);
    assert_eq!(invocations[1][0], b"--settings");
    assert_eq!(invocations[1].last(), Some(&b"run".to_vec()));
}

#[test]
fn unselected_passthrough_never_runs_the_version_probe() {
    let harness = Harness::new();
    let output = harness
        .companion_profile_command()
        .arg("run")
        .env("CLAUDE_CONFIG_DIR", "ambient-config")
        .env("CLAUDE_CODE_OAUTH_TOKEN", "ambient-token")
        .output()
        .expect("wrapper");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(78), "{stderr}");
    assert!(stderr.contains("error[Config]"), "{stderr}");
    assert!(
        stderr.contains("account=none, profile=companion"),
        "{stderr}"
    );
    assert!(
        stderr.contains("child launch is not bound to a complete session"),
        "{stderr}"
    );
    for section in ["Where:", "Why:", "Hint:"] {
        assert!(stderr.contains(section), "missing {section}:\n{stderr}");
    }
    assert!(
        stderr.contains("claude-session-rs account login"),
        "{stderr}"
    );
    assert!(output.stdout.is_empty());
    assert!(!harness.record_dir().join("invocations").exists());
    assert!(!harness.state().join("state/last-account").exists());
}

#[test]
fn account_grammar_is_wrapper_owned_only_in_leading_position() {
    let harness = Harness::new();
    harness.assert_command().arg("account").assert().code(64);
    harness
        .assert_command()
        .args(["account", "unknown"])
        .assert()
        .code(64);
    assert!(
        harness
            .bound_command()
            .args(["--", "account", "list"])
            .status()
            .expect("wrapper")
            .success()
    );
    assert_eq!(
        &read_nul(&harness.record_dir().join("argv"))[3..],
        &[b"account".to_vec(), b"list".to_vec()]
    );
}

/// Requested help is a result, not a diagnostic: standard output, exit `0`,
/// and `help <verb>` prints what `<verb> --help` prints
/// (`docs/reference/cli-surface.md#help`). Malformed invocations of the same
/// namespace stay diagnostics on standard error at `Usage`.
#[test]
fn requested_account_help_is_a_result_and_malformed_account_grammar_is_not() {
    let harness = Harness::new();
    let namespace = harness
        .command()
        .args(["account", "--help"])
        .output()
        .expect("account --help");
    assert!(namespace.status.success());
    assert!(namespace.stderr.is_empty());
    let text = String::from_utf8(namespace.stdout.clone()).expect("text");
    assert!(
        text.starts_with("Manage durable child-owned accounts"),
        "{text}"
    );
    assert!(text.contains("Usage: claude-session-rs account"));
    assert!(text.contains("login"));
    assert!(text.contains("list"));

    // The verb spelling of the same surface prints the same bytes.
    let verb = harness
        .command()
        .args(["help", "account"])
        .output()
        .expect("help account");
    assert!(verb.status.success());
    assert_eq!(verb.stdout, namespace.stdout);

    // A subcommand answers with its own help rather than the namespace's.
    for (subcommand, headline) in [
        ("list", "List locally discovered accounts"),
        ("login", "Delegate native saved-login setup to the child"),
    ] {
        let output = harness
            .command()
            .args(["account", subcommand, "--help"])
            .output()
            .expect("subcommand help");
        assert!(output.status.success());
        let text = String::from_utf8(output.stdout).expect("text");
        assert!(text.starts_with(headline), "{text}");
        assert!(text.contains(&format!("Usage: claude-session-rs account {subcommand}")));
    }

    // Nothing about the malformed cases changed: they stay diagnostics, and
    // the bare verb's diagnostic is still the verb's own help, so the closed
    // subcommand set is discoverable where the user meets the failure.
    for (arguments, expected) in [
        (
            vec!["account"],
            vec!["Usage: claude-session-rs account", "login", "list"],
        ),
        // An unrecognized subcommand carries a nearest match instead, because
        // the parser's subcommand set is closed and wholly wrapper-owned.
        (
            vec!["account", "lst"],
            vec![
                "unrecognized subcommand 'lst'",
                "a similar subcommand exists: 'list'",
            ],
        ),
    ] {
        let output = harness
            .command()
            .args(&arguments)
            .output()
            .expect("malformed");
        assert_eq!(output.status.code(), Some(64), "{arguments:?}");
        assert!(output.stdout.is_empty(), "{arguments:?}");
        let diagnostic = String::from_utf8(output.stderr).expect("text");
        for line in expected {
            assert!(
                diagnostic.contains(line),
                "{arguments:?} lost {line}: {diagnostic}"
            );
        }
    }

    // The child never runs for any of it.
    assert!(!harness.record_dir().join("argv").exists());
}

/// An interactive login can take minutes, so the account tree it commits into
/// is revalidated after the child returns rather than trusted from before it
/// started (`xdg-storage.md#how-a-path-is-validated`).
#[test]
fn native_login_revalidates_the_account_tree_before_committing() {
    let harness = Harness::new();
    let elsewhere = harness.root().join("elsewhere");
    fs::create_dir_all(&elsewhere).expect("fixture");
    fs::write(elsewhere.join(".credentials.json"), b"outside").expect("fixture");
    let output = harness
        .terminal_command("account login work")
        .env("CS_TEST_SWAP_CONFIG", &elsewhere)
        .output()
        .expect("terminal login");
    assert_eq!(
        output.status.code(),
        Some(77),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let terminal = String::from_utf8_lossy(&output.stdout);
    assert!(
        terminal.contains("error[Permission]"),
        "a swapped directory is a storage defect, not a login failure: {terminal}"
    );
    assert!(
        !harness
            .state()
            .join("accounts/work/auth-mode.json")
            .exists(),
        "no metadata may be committed over a credential outside the account tree"
    );
}

// --- Slice 013: the token account lifecycle -------------------------------

/// Runs one wrapper invocation and returns its parsed JSON report.
fn account_json(harness: &Harness, arguments: &[&str]) -> serde_json::Value {
    let output = harness.command().args(arguments).output().expect("wrapper");
    assert!(
        output.status.success(),
        "{:?} failed: {}",
        arguments,
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("json")
}

/// Acceptance line 1: standard input is one of the two permitted sources, and
/// the commit lands under the credential lock at the documented modes.
#[test]
fn token_ingest_from_stdin_commits_the_pair_privately() {
    let harness = Harness::new();
    let output = harness
        .assert_command()
        .args(["account", "login", "work", "--token", "--stdin", "--json"])
        .write_stdin("sk-ingest-value\n")
        .output()
        .expect("token login");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let account = harness.state().join("accounts/work");
    assert_eq!(
        fs::read(account.join("oauth-token")).expect("token file"),
        b"sk-ingest-value"
    );
    assert_eq!(mode(&account.join("oauth-token")), 0o600);
    assert_eq!(mode(&account.join("auth-mode.json")), 0o600);
    let metadata: serde_json::Value =
        serde_json::from_slice(&fs::read(account.join("auth-mode.json")).expect("metadata file"))
            .expect("metadata");
    assert_eq!(metadata["mode"], "token");
    assert_eq!(
        metadata["fingerprint"],
        support::fingerprint(b"sk-ingest-value")
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(report["mode"], "token");
    assert_eq!(
        report["fingerprint"],
        support::fingerprint(b"sk-ingest-value")
    );
    // The lock sentinel survives its own transaction, and lives beside the
    // account rather than inside it: it is a handle rather than a claim, and
    // deleting it would let one holder destroy the file another is about to
    // lock.
    assert!(
        harness.state().join("accounts/.work.lock").is_file(),
        "the sentinel belongs beside the account it guards"
    );
    assert!(!account.join(".credentials.lock").exists());
}

/// The candidate reaches the child through its environment and never through
/// argv, so it cannot appear in a process listing.
#[test]
fn the_verification_probe_carries_the_candidate_in_the_environment_only() {
    let harness = Harness::new();
    assert!(
        harness
            .assert_command()
            .args(["account", "login", "work", "--token", "--stdin"])
            .write_stdin("sk-probe-value\n")
            .output()
            .expect("token login")
            .status
            .success()
    );
    let argv = read_nul(&harness.record_dir().join("argv"));
    assert_eq!(
        &argv[1..],
        &[b"auth".to_vec(), b"status".to_vec(), b"--json".to_vec()]
    );
    assert!(
        !argv.iter().any(|value| value == b"sk-probe-value"),
        "the token must never reach argv"
    );
    assert_eq!(
        fs::read(harness.record_dir().join("probe-token")).expect("probe record"),
        b"sk-probe-value",
        "the probe answers about the candidate, so the candidate must reach it"
    );
}

/// A candidate the child refuses never replaces anything, and a failed first
/// login leaves no account behind.
#[test]
fn a_refused_candidate_commits_nothing_and_removes_a_first_account() {
    let harness = Harness::new();
    let output = harness
        .assert_command()
        .args(["account", "login", "work", "--token", "--stdin"])
        .env("CS_TEST_PROBE_EXIT", "1")
        .write_stdin("sk-rejected\n")
        .output()
        .expect("token login");
    assert_eq!(output.status.code(), Some(77));
    assert!(
        !harness.state().join("accounts/work").exists(),
        "a failed first login removes the account it created"
    );
}

/// Every refused shape is a usage error raised before anything is written, and
/// the diagnostic never quotes what it refused.
#[test]
fn a_malformed_pasted_token_is_refused_without_quoting_it() {
    for (input, why) in [
        ("", "no token was supplied"),
        ("   \n", "no token was supplied"),
        (
            "first\nsecond\n",
            "a token is one line, and more than one arrived",
        ),
    ] {
        let harness = Harness::new();
        let output = harness
            .assert_command()
            .args(["account", "login", "work", "--token", "--stdin"])
            .write_stdin(input)
            .output()
            .expect("token login");
        assert_eq!(output.status.code(), Some(64), "input {input:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains(why), "input {input:?}: {stderr}");
        assert!(
            !stderr.contains("first"),
            "the diagnostic must not quote the refused input: {stderr}"
        );
        assert!(!harness.state().join("accounts/work").exists());
    }
}

/// Acceptance line 1: without a terminal and without the documented escape, the
/// verb stops before any side effect and names the escape.
#[test]
fn token_entry_without_a_terminal_refuses_before_any_side_effect() {
    let harness = Harness::new();
    let output = harness
        .detached_command(&["account", "login", "work", "--token"])
        .output()
        .expect("detached login");
    assert_eq!(output.status.code(), Some(69));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--token --stdin"), "{stderr}");
    assert!(!harness.state().join("accounts/work").exists());
    assert!(!harness.record_dir().join("argv").exists());
}

/// Acceptance line 3: the floor guards shared-login refresh coordination, which
/// token mode does not use, so a below-floor child still launches.
#[test]
fn a_token_launch_is_not_blocked_by_the_login_mode_version_floor() {
    let harness = Harness::new();
    harness.initialize_token("work", b"sk-launch-value", b"sk-launch-value");
    let output = harness
        .companion_profile_command()
        .args(["--account", "work", "run"])
        .env("CS_TEST_VERSION_STDOUT", "2.1.210 (Claude Code)\n")
        .output()
        .expect("launch");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let invocations = read_invocations(&harness.record_dir().join("invocations"));
    assert_eq!(invocations.len(), 1, "token mode runs no version probe");
    assert_eq!(invocations[0][0], b"--settings");
    assert_eq!(invocations[0].last(), Some(&b"run".to_vec()));
    let environ = read_nul(&harness.record_dir().join("environ"));
    let index = environ
        .iter()
        .position(|value| value == b"CLAUDE_CODE_OAUTH_TOKEN")
        .expect("the token is injected");
    assert_eq!(environ[index + 1], b"sk-launch-value");
}

/// Acceptance line 2: ambient authentication is preserved and warned about, and
/// the stored mode is untouched.
#[test]
fn ambient_authentication_is_preserved_and_warned_without_changing_the_mode() {
    let harness = Harness::new();
    harness.initialize_token("work", b"sk-ambient-value", b"sk-ambient-value");
    let output = harness
        .companion_profile_command()
        .args(["--account", "work", "run"])
        .env("ANTHROPIC_API_KEY", "ambient-key")
        .output()
        .expect("launch");
    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("ANTHROPIC_API_KEY outranks the selected account"),
        "{stderr}"
    );
    let environ = read_nul(&harness.record_dir().join("environ"));
    let index = environ
        .iter()
        .position(|value| value == b"ANTHROPIC_API_KEY")
        .expect("the ambient key is never stripped");
    assert_eq!(environ[index + 1], b"ambient-key");
    let metadata: serde_json::Value = serde_json::from_slice(
        &fs::read(harness.state().join("accounts/work/auth-mode.json")).expect("metadata file"),
    )
    .expect("metadata");
    assert_eq!(metadata["mode"], "token", "the stored mode is unchanged");
}

/// Acceptance line 4: mode-aware health, and the crash-between-renames window
/// reported rather than papered over.
#[test]
fn status_reports_mode_aware_health_and_metadata_consistency() {
    let harness = Harness::new();
    harness.initialize_token("consistent", b"sk-good", b"sk-good");
    harness.initialize_token("torn", b"sk-actual", b"sk-described");
    harness.initialize_login("saved");

    let consistent = account_json(&harness, &["account", "status", "consistent", "--json"]);
    assert_eq!(consistent["mode"], "token");
    assert_eq!(consistent["usable"], true);
    assert_eq!(consistent["metadata_consistent"], true);
    assert_eq!(consistent["fingerprint"], support::fingerprint(b"sk-good"));
    assert!(consistent["age_seconds"].is_number());
    assert_eq!(consistent["estimated_expiry"], "2027-08-11T00:00:00Z");
    assert_eq!(consistent["child_login_present"], false);
    assert_eq!(consistent["child_probe"]["status"], "ok");

    let torn = account_json(&harness, &["account", "status", "torn", "--json"]);
    assert_eq!(torn["metadata_consistent"], false);
    assert_eq!(
        torn["fingerprint"],
        support::fingerprint(b"sk-actual"),
        "the reported fingerprint identifies the credential actually in force"
    );
    assert!(
        torn["age_seconds"].is_null() && torn["estimated_expiry"].is_null(),
        "a mint time that is not the token's computes nothing: {torn}"
    );
    assert_eq!(torn["usable"], true, "the token itself still works");

    let saved = account_json(&harness, &["account", "status", "saved", "--json"]);
    assert_eq!(saved["mode"], "login");
    assert_eq!(saved["usable"], true);
    assert_eq!(saved["child_login_present"], true);
    assert!(
        saved["fingerprint"].is_null() && saved["metadata_consistent"].is_null(),
        "login mode has no wrapper-owned secret to describe: {saved}"
    );
}

/// A status request about an account that does not exist is the one thing this
/// inspection verb fails on, and it is `NoInput` rather than a report.
#[test]
fn status_for_an_absent_account_reports_no_input() {
    let harness = Harness::new();
    let output = harness
        .command()
        .args(["account", "status", "missing", "--json"])
        .output()
        .expect("status");
    assert_eq!(output.status.code(), Some(66));
}

/// Token-over-login shadowing is data in this report, because shadowing is its
/// subject; everywhere else it is prose on standard error.
#[test]
fn status_carries_shadowing_as_data_rather_than_prose() {
    let harness = Harness::new();
    harness.initialize_token("both", b"sk-token", b"sk-token");
    fs::write(
        harness
            .state()
            .join("accounts/both/config/.credentials.json"),
        b"child-owned-fixture",
    )
    .expect("saved login fixture");
    let status = account_json(&harness, &["account", "status", "both", "--json"]);
    let warnings = status["warnings"].as_array().expect("warnings array");
    assert!(
        warnings.iter().any(|warning| warning
            .as_str()
            .is_some_and(|text| text.contains("the stored token outranks the saved login"))),
        "{status}"
    );
}

/// Acceptance line 5: only local state goes, in the order whose first unlink is
/// the commit, and the report says what is still live upstream.
#[test]
fn remove_deletes_only_local_state_and_states_that_nothing_was_revoked() {
    let harness = Harness::new();
    harness.initialize_token("work", b"sk-remove", b"sk-remove");
    harness.initialize_token("other", b"sk-keep", b"sk-keep");
    let marker = harness.state().join("state/last-account");
    fs::create_dir_all(marker.parent().expect("state directory")).expect("marker directory");
    fs::write(&marker, b"work").expect("marker fixture");
    fs::set_permissions(&marker, fs::Permissions::from_mode(0o600)).expect("marker mode");

    let output = harness
        .command()
        .args(["account", "remove", "work", "--yes", "--json"])
        .output()
        .expect("remove");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(report["removed"], true);
    assert_eq!(report["mode"], "token");
    assert_eq!(report["marker_cleared"], true);
    assert!(!harness.state().join("accounts/work").exists());
    assert!(!marker.exists());
    assert!(
        harness.state().join("accounts/other/oauth-token").is_file(),
        "removal takes one account and nothing else"
    );
    assert!(
        harness.state().join("accounts").is_dir(),
        "the collection survives its last account"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not upstream revocation"),
        "the one place this fact exists is a sentence, and --json must not delete it: {stderr}"
    );
}

/// Acceptance line 5: without consent and without a terminal, removal stops
/// before any side effect; with `--json` the schema changes and nothing else.
#[test]
fn remove_without_consent_or_a_terminal_changes_nothing() {
    let harness = Harness::new();
    harness.initialize_token("work", b"sk-keep-me", b"sk-keep-me");
    for arguments in [
        vec!["account", "remove", "work"],
        vec!["account", "remove", "work", "--json"],
    ] {
        let output = harness
            .detached_command(&arguments)
            .output()
            .expect("detached remove");
        assert_eq!(output.status.code(), Some(69), "{arguments:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("--yes"), "{arguments:?}: {stderr}");
        assert_eq!(
            fs::read(harness.state().join("accounts/work/oauth-token")).expect("token file"),
            b"sk-keep-me"
        );
    }
}

/// The verification probe must answer about the candidate, so it clears the
/// mechanisms that would otherwise outrank it. Without this an invalid token
/// verifies whenever an API key happens to be exported.
#[test]
fn the_verification_probe_clears_the_credentials_that_outrank_its_candidate() {
    let harness = Harness::new();
    let mut command = harness.assert_command();
    for variable in [
        "ANTHROPIC_API_KEY",
        "ANTHROPIC_AUTH_TOKEN",
        "CLAUDE_CODE_USE_BEDROCK",
        "CLAUDE_CODE_USE_VERTEX",
        "CLAUDE_CODE_USE_FOUNDRY",
    ] {
        command.env(variable, "ambient");
    }
    let output = command
        .args(["account", "login", "work", "--token", "--stdin"])
        .write_stdin("sk-isolated\n")
        .output()
        .expect("token login");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    // The probe is this account's only child invocation, so the recorded
    // environment is the probe's.
    let environ = read_nul(&harness.record_dir().join("environ"));
    for variable in [
        "ANTHROPIC_API_KEY",
        "ANTHROPIC_AUTH_TOKEN",
        "CLAUDE_CODE_USE_BEDROCK",
        "CLAUDE_CODE_USE_VERTEX",
        "CLAUDE_CODE_USE_FOUNDRY",
    ] {
        assert!(
            !environ.iter().any(|value| value == variable.as_bytes()),
            "{variable} would decide the probe's answer instead of the candidate"
        );
    }
    let index = environ
        .iter()
        .position(|value| value == b"CLAUDE_CODE_OAUTH_TOKEN")
        .expect("the candidate reaches the probe");
    assert_eq!(environ[index + 1], b"sk-isolated");
}

/// A failed login must not delete an account a concurrent run committed while
/// its child was still going.
#[test]
fn a_failed_first_login_leaves_an_account_another_run_committed() {
    let harness = Harness::new();
    let output = harness
        .terminal_command("account login work")
        .env("CS_TEST_COMMIT_METADATA", "1")
        .env("CS_TEST_EXIT", "1")
        .output()
        .expect("terminal login");
    assert!(!output.status.success());
    assert!(
        harness
            .state()
            .join("accounts/work/auth-mode.json")
            .is_file(),
        "the committed account belongs to the run that finished, not to this one"
    );
}

/// A multi-line paste is refused rather than truncated, and its remainder is
/// consumed rather than left queued for the user's shell.
#[test]
fn a_multi_line_paste_into_the_terminal_is_refused() {
    let harness = Harness::new();
    let mut command = harness.terminal_command("account login work --token");
    command
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = command.spawn().expect("terminal login");
    // The paste has to arrive after the prompt. Engaging echo-off flushes
    // type-ahead by design, so anything written before then is discarded and
    // the case under test would never be reached.
    let mut transcript = Vec::new();
    {
        use std::io::{Read as _, Write as _};
        let mut stdout = child.stdout.take().expect("stdout");
        let mut byte = [0_u8; 1];
        while !transcript.ends_with(b"not echoed): ") {
            assert_eq!(
                stdout.read(&mut byte).expect("prompt"),
                1,
                "the prompt never arrived: {}",
                String::from_utf8_lossy(&transcript)
            );
            transcript.push(byte[0]);
        }
        let mut stdin = child.stdin.take().expect("stdin");
        stdin
            .write_all(b"sk-first-line\nsk-second-line\n")
            .expect("paste");
        drop(stdin);
        stdout.read_to_end(&mut transcript).expect("transcript");
    }
    child.wait().expect("terminal login");
    let terminal = String::from_utf8_lossy(&transcript);
    assert!(
        terminal.contains("a token is one line, and more than one arrived"),
        "the second line must be refused rather than silently dropped: {terminal}"
    );
    assert!(
        !harness.state().join("accounts/work/oauth-token").exists(),
        "nothing may be stored from a refused paste"
    );
}

/// The lock sentinel outlives the account it guarded, because destroying it is
/// what let a third arrival recreate the name and lock an inode nobody held.
#[test]
fn removal_leaves_the_lock_sentinel_and_no_visible_account() {
    let harness = Harness::new();
    harness.initialize_token("work", b"sk-outlived", b"sk-outlived");
    assert!(
        harness
            .command()
            .args(["account", "remove", "work", "--yes", "--json"])
            .output()
            .expect("remove")
            .status
            .success()
    );
    assert!(!harness.state().join("accounts/work").exists());
    let sentinel = harness.state().join("accounts/.work.lock");
    assert!(
        sentinel.is_file(),
        "the sentinel is never deleted, so acquisition and removal exclude each other throughout"
    );
    assert_eq!(mode(&sentinel), 0o600);
    // Invisible twice over: the walk takes directories, and an identifier
    // cannot begin with a dot.
    let value = list_json(&harness);
    assert_eq!(value["accounts"].as_array().expect("accounts").len(), 0);
    // And it does not obstruct a later account of the same name.
    assert!(
        harness
            .assert_command()
            .args(["account", "login", "work", "--token", "--stdin"])
            .write_stdin("sk-reused\n")
            .output()
            .expect("token login")
            .status
            .success()
    );
    assert_eq!(
        fs::read(harness.state().join("accounts/work/oauth-token")).expect("token file"),
        b"sk-reused"
    );
}

/// The interrupt key cancels the prompt instead of killing the process while
/// terminal echo is off. Reaching this at all proves `ISIG` was cleared: with
/// it on the byte would have become a signal and never been read.
#[test]
fn an_interrupt_at_the_token_prompt_cancels_instead_of_killing_the_process() {
    let harness = Harness::new();
    let mut command = harness.terminal_command("account login work --token");
    command
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = command.spawn().expect("terminal login");
    let mut transcript = Vec::new();
    {
        use std::io::{Read as _, Write as _};
        let mut stdout = child.stdout.take().expect("stdout");
        let mut byte = [0_u8; 1];
        while !transcript.ends_with(b"not echoed): ") {
            assert_eq!(stdout.read(&mut byte).expect("prompt"), 1, "prompt");
            transcript.push(byte[0]);
        }
        let mut stdin = child.stdin.take().expect("stdin");
        stdin.write_all(b"\x03\n").expect("interrupt");
        drop(stdin);
        stdout.read_to_end(&mut transcript).expect("transcript");
    }
    let status = child.wait().expect("terminal login");
    let terminal = String::from_utf8_lossy(&transcript);
    assert!(
        terminal.contains("entry was cancelled at the prompt"),
        "the interrupt must be answered rather than signalled: {terminal}"
    );
    assert_eq!(
        status.code(),
        Some(64),
        "a cancelled entry is a refusal the wrapper reports, not a death"
    );
    assert!(!harness.state().join("accounts/work").exists());
}
