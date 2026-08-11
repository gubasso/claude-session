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
            "\"fingerprint\":\"private\"}\n"
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
        .command()
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
        .command()
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
            .command()
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
        .command()
        .args(["--account", "work", "run"])
        .env("CS_TEST_VERSION_STDOUT", "2.1.210\n")
        .output()
        .expect("wrapper");
    assert_eq!(output.status.code(), Some(69));
    assert!(String::from_utf8_lossy(&output.stderr).contains(concat!(
        "The resolved `claude` reports 2.1.210, below the 2.1.211 this ",
        "wrapper is designed against. Upgrade it before using a ",
        "saved-login account."
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
        .command()
        .args(["--account", "work", "run"])
        .env("CS_TEST_VERSION_STDOUT", "garbage\n")
        .output()
        .expect("wrapper");
    assert_eq!(output.status.code(), Some(69));
    assert!(String::from_utf8_lossy(&output.stderr).contains(concat!(
        "The resolved `claude` reports unavailable, below the 2.1.211 this ",
        "wrapper is designed against. Upgrade it before using a ",
        "saved-login account."
    )));
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
        .command()
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
        .command()
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
        .command()
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
            .command()
            .args(["--account", "work", "run"])
            .env("CS_TEST_VERSION_STDOUT", "2.1.211\n")
            .status()
            .expect("wrapper")
            .success()
    );
    let marker = harness.state().join("state/last-account");
    assert_eq!(fs::read(&marker).expect("marker"), b"work");
    assert_eq!(
        read_invocations(&harness.record_dir().join("invocations")),
        vec![vec![b"--version".to_vec()], vec![b"run".to_vec()]],
        "the floor probe must precede the exec"
    );
}

#[test]
fn unselected_passthrough_never_runs_the_version_probe() {
    let harness = Harness::new();
    assert!(
        harness
            .command()
            .arg("run")
            .env("CLAUDE_CONFIG_DIR", "ambient-config")
            .env("CLAUDE_CODE_OAUTH_TOKEN", "ambient-token")
            .status()
            .expect("wrapper")
            .success()
    );
    let environment = read_nul(&harness.record_dir().join("environ"));
    assert!(
        environment
            .windows(2)
            .any(|pair| pair[0] == b"CLAUDE_CONFIG_DIR" && pair[1] == b"ambient-config")
    );
    assert!(
        environment
            .windows(2)
            .any(|pair| pair[0] == b"CLAUDE_CODE_OAUTH_TOKEN" && pair[1] == b"ambient-token")
    );
    assert_eq!(
        read_invocations(&harness.record_dir().join("invocations")),
        vec![vec![b"run".to_vec()]],
        "the launch is the only child invocation, so no version probe ran"
    );
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
            .command()
            .args(["--", "account", "list"])
            .status()
            .expect("wrapper")
            .success()
    );
    assert_eq!(
        &read_nul(&harness.record_dir().join("argv"))[1..],
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
    assert!(text.contains("Usage: claude-session account"));
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
        assert!(text.contains(&format!("Usage: claude-session account {subcommand}")));
    }

    // Nothing about the malformed cases changed: they stay diagnostics, and
    // the bare verb's diagnostic is still the verb's own help, so the closed
    // subcommand set is discoverable where the user meets the failure.
    for (arguments, expected) in [
        (
            vec!["account"],
            vec!["Usage: claude-session account", "login", "list"],
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
