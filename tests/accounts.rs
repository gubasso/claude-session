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
    fs::remove_dir_all(accounts.join("linked/config")).expect("fixture");
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
    // The same facts, said rather than labelled: which account, how it signs
    // in, that it is usable, and which one this run would use and why.
    for fact in [
        "work — signs in with a saved login",
        "[pass]",
        "This run would use work, because you named it with --account.",
    ] {
        assert!(text.contains(fact), "{fact} missing from {text}");
    }
    for field in ["selection_source:", "usable:", "mode:", "selected:"] {
        assert!(!text.contains(field), "{field} survives in {text}");
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
        .detached_command(&["account", "login", "work", "--profile", "companion"])
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
        .terminal_command("account login work --profile companion")
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
        .terminal_command("account login work --profile companion")
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
        .terminal_command("account login work --profile companion")
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
        .terminal_command(&format!(
            "account login work --profile companion --json > {}",
            stdout.display()
        ))
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
        .terminal_command(&format!(
            "account login work --profile companion --json > {}",
            stdout.display()
        ))
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
    // The same acceptance sentence binds the launch environment: this
    // terminal's session directory is the child's configuration directory, the
    // account's directory is its credential store, and no wrapper token
    // variable is set.
    let environment = read_nul(&harness.record_dir().join("environ"));
    let sessions = harness.state().join("accounts/work/sessions");
    let config = harness.state().join("accounts/work/config");
    assert!(environment.windows(2).any(|pair| {
        pair[0] == b"CLAUDE_CONFIG_DIR" && pair[1].starts_with(bytes(sessions.as_os_str()))
    }));
    assert!(environment.windows(2).any(|pair| {
        pair[0] == b"CLAUDE_SECURESTORAGE_CONFIG_DIR" && pair[1] == bytes(config.as_os_str())
    }));
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
    let sessions = harness.state().join("accounts/work/sessions");
    let config = harness.state().join("accounts/work/config");
    assert!(environment.windows(2).any(|pair| {
        pair[0] == b"CLAUDE_CONFIG_DIR" && pair[1].starts_with(bytes(sessions.as_os_str()))
    }));
    assert!(environment.windows(2).any(|pair| {
        pair[0] == b"CLAUDE_SECURESTORAGE_CONFIG_DIR" && pair[1] == bytes(config.as_os_str())
    }));
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
    // Compared with the wrap collapsed, because the remediation is now laid
    // out for a reader rather than emitted as one line.
    let flowed = String::from_utf8_lossy(&output.stderr)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        flowed.contains(concat!(
            "Upgrade claude to 2.1.211 or newer before using a saved-login account. ",
            "Token accounts are unaffected and still work below that version."
        )),
        "{flowed}"
    );
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
    let flowed = stderr.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        flowed.contains(concat!(
            "Upgrade claude to 2.1.211 or newer before using a saved-login account. ",
            "Token accounts are unaffected and still work below that version."
        )),
        "{flowed}"
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
    // The subject names what resolved and what did not, in a sentence. It
    // carried `account=none, profile=companion` until the sweep reached the
    // diagnostic, which is the one machine shape a human surface may not have.
    assert!(
        support::flowed(&stderr).contains("resolved the \"companion\" profile but no account"),
        "{stderr}"
    );
    assert!(!stderr.contains("profile=companion"), "{stderr}");
    assert!(
        stderr.contains("child launch is not bound to a complete session"),
        "{stderr}"
    );
    // The labelled triple is gone; what a reader needs from it is not.
    for section in ["Where:", "Why:", "Hint:"] {
        assert!(!stderr.contains(section), "{section} survives:\n{stderr}");
    }
    assert!(stderr.contains("What to do:"), "{stderr}");
    assert!(stderr.contains("This concerns"), "{stderr}");
    assert!(stderr.contains("claude-session account login"), "{stderr}");
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
        .terminal_command("account login work --profile companion")
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
        .args([
            "account",
            "login",
            "work",
            "--profile",
            "companion",
            "--token",
            "--stdin",
            "--json",
        ])
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
            .args([
                "account",
                "login",
                "work",
                "--profile",
                "companion",
                "--token",
                "--stdin"
            ])
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
        .args([
            "account",
            "login",
            "work",
            "--profile",
            "companion",
            "--token",
            "--stdin",
        ])
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
            .args([
                "account",
                "login",
                "work",
                "--profile",
                "companion",
                "--token",
                "--stdin",
            ])
            .write_stdin(input)
            .output()
            .expect("token login");
        assert_eq!(output.status.code(), Some(64), "input {input:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.to_lowercase().contains(why),
            "input {input:?}: {stderr}"
        );
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
        .detached_command(&[
            "account",
            "login",
            "work",
            "--profile",
            "companion",
            "--token",
        ])
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
        .args([
            "account",
            "login",
            "work",
            "--profile",
            "companion",
            "--token",
            "--stdin",
        ])
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
        .terminal_command("account login work --profile companion")
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
    let mut command = harness.terminal_command("account login work --profile companion --token");
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
        terminal
            .to_lowercase()
            .contains("a token is one line, and more than one arrived"),
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
            .args([
                "account",
                "login",
                "work",
                "--profile",
                "companion",
                "--token",
                "--stdin"
            ])
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
    let mut command = harness.terminal_command("account login work --profile companion --token");
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
        terminal
            .to_lowercase()
            .contains("entry was cancelled at the prompt"),
        "the interrupt must be answered rather than signalled: {terminal}"
    );
    assert_eq!(
        status.code(),
        Some(64),
        "a cancelled entry is a refusal the wrapper reports, not a death"
    );
    assert!(!harness.state().join("accounts/work").exists());
}

/// Creating an account is where the profile choice becomes explicit
/// ([ADR-0096](../docs/decisions/ADR-0096-bind-a-profile-to-an-account.md)), so
/// the report names the profile and the layer that supplied it whether the user
/// typed it or a default answered.
#[test]
fn a_login_names_the_profile_it_bound_and_where_it_came_from() {
    let harness = Harness::new();
    harness.write_piece("companion", "{}\n");
    harness.write_profile("companion", "layers:\n  - companion\n");
    let output = harness
        .assert_command()
        .args([
            "account",
            "login",
            "work",
            "--profile",
            "companion",
            "--token",
            "--stdin",
            "--json",
        ])
        .write_stdin("sk-bound\n")
        .output()
        .expect("login");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(value["profile"], "companion");
    assert_eq!(value["profile_source"], "cli");
    assert_eq!(value["profile_present"], true);
    let binding: serde_json::Value = serde_json::from_slice(
        &fs::read(harness.state().join("accounts/work/profile.json")).expect("binding"),
    )
    .expect("binding json");
    assert_eq!(binding["profile"], "companion");
}

#[test]
fn a_login_falls_back_to_the_configured_default_and_says_so() {
    let harness = Harness::new();
    harness.write_profile("companion", "layers:\n  - companion\n");
    harness.write_piece("companion", "{}\n");
    fs::write(
        harness.config_base().join("config.toml"),
        "default_profile = \"companion\"\n",
    )
    .expect("user config");
    let output = harness
        .assert_command()
        .args(["account", "login", "work", "--token", "--stdin", "--json"])
        .write_stdin("sk-defaulted\n")
        .output()
        .expect("login");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(value["profile"], "companion");
    assert_eq!(value["profile_source"], "user-config");
}

/// The refusal comes before the child and before any directory is made, so a
/// login that cannot say which profile it binds costs nothing to retry.
#[test]
fn a_login_with_no_profile_anywhere_is_refused_as_configuration() {
    let harness = Harness::new();
    let output = harness
        .assert_command()
        .args(["account", "login", "work", "--token", "--stdin"])
        .write_stdin("sk-unbound\n")
        .output()
        .expect("login");
    assert_eq!(output.status.code(), Some(78));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--profile"), "{stderr}");
    assert!(stderr.contains("default_profile"), "{stderr}");
    assert!(!harness.state().join("accounts/work").exists());
}

#[test]
fn binding_records_the_profile_and_every_account_surface_reports_it() {
    let harness = Harness::new();
    harness.initialize_token("work", b"sk-bound", b"sk-bound");
    harness.write_profile("companion", "layers:\n  - companion\n");
    harness.write_piece("companion", "{}\n");
    harness
        .assert_command()
        .args(["account", "bind", "work", "--profile", "companion"])
        .assert()
        .success();
    assert_eq!(list_json(&harness)["accounts"][0]["profile"], "companion");
    let output = harness
        .command()
        .args(["account", "status", "work", "--json"])
        .output()
        .expect("status");
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(value["profile"], "companion");
    assert_eq!(value["profile_present"], true);
}

/// A typo must not unbind a working account, so the name is checked before
/// anything is written
/// ([ADR-0097](../docs/decisions/ADR-0097-rebind-a-profile-without-re-authenticating.md)).
#[test]
fn binding_a_profile_without_a_document_keeps_the_previous_binding() {
    let harness = Harness::new();
    harness.initialize_token("work", b"sk-bound", b"sk-bound");
    harness.write_profile("companion", "layers:\n  - companion\n");
    harness.write_piece("companion", "{}\n");
    harness
        .assert_command()
        .args(["account", "bind", "work", "--profile", "companion"])
        .assert()
        .success();
    let output = harness
        .command()
        .args(["account", "bind", "work", "--profile", "absent"])
        .output()
        .expect("bind");
    assert_eq!(output.status.code(), Some(66));
    assert_eq!(list_json(&harness)["accounts"][0]["profile"], "companion");
}

/// The rung sits under the project layer and over user configuration, which is
/// what makes a per-tree override still win and a global default still lose.
#[test]
fn the_binding_outranks_user_configuration_and_yields_to_the_project_layer() {
    let harness = Harness::new();
    harness.initialize_token("work", b"sk-bound", b"sk-bound");
    for name in ["bound", "global", "tree"] {
        harness.write_profile(name, &format!("layers:\n  - {name}\n"));
        harness.write_piece(name, "{}\n");
    }
    fs::write(
        harness.config_base().join("config.toml"),
        "default_profile = \"global\"\n",
    )
    .expect("user config");
    harness
        .assert_command()
        .args(["account", "bind", "work", "--profile", "bound"])
        .assert()
        .success();
    let profile = |harness: &Harness| -> String {
        let output = harness
            .command()
            .args(["--account", "work", "config", "--json"])
            .output()
            .expect("config");
        let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
        value["profile"]["name"]
            .as_str()
            .expect("a resolved profile")
            .to_owned()
    };
    assert_eq!(profile(&harness), "bound");
    fs::write(
        harness.root().join(".claude-session.toml"),
        "default_profile = \"tree\"\n",
    )
    .expect("project config");
    // Clear git's own environment. A git hook exports GIT_DIR and GIT_WORK_TREE
    // to everything it runs, so under `git push` this `init` would adopt the
    // repository's git directory instead of creating one in the fixture, and the
    // project layer would have no tree boundary to be discovered from. In a
    // linked worktree GIT_DIR is absolute, so it resolves from the fixture's
    // directory and the test fails; in the main checkout it is relative, does
    // not resolve, and the test passes. Only clearing makes the lane honest.
    std::process::Command::new("git")
        .args(["init", "-q"])
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_COMMON_DIR")
        .env_remove("GIT_CEILING_DIRECTORIES")
        .current_dir(harness.root())
        .status()
        .expect("git init");
    assert_eq!(profile(&harness), "tree");
}

#[test]
fn removing_an_account_takes_its_binding_with_it() {
    let harness = Harness::new();
    harness.initialize_token("work", b"sk-bound", b"sk-bound");
    harness.write_profile("companion", "layers:\n  - companion\n");
    harness.write_piece("companion", "{}\n");
    harness
        .assert_command()
        .args(["account", "bind", "work", "--profile", "companion"])
        .assert()
        .success();
    harness
        .assert_command()
        .args(["account", "remove", "work", "--yes"])
        .assert()
        .success();
    assert!(!harness.state().join("accounts/work/profile.json").exists());
}

/// A bound profile whose document has gone travels as a warning value, like the
/// shadowing conditions do, rather than as prose only.
#[test]
fn a_bound_profile_with_no_document_is_carried_as_data() {
    let harness = Harness::new();
    harness.initialize_token("work", b"sk-bound", b"sk-bound");
    harness.write_profile("companion", "layers:\n  - companion\n");
    harness.write_piece("companion", "{}\n");
    harness
        .assert_command()
        .args(["account", "bind", "work", "--profile", "companion"])
        .assert()
        .success();
    fs::remove_file(harness.config_base().join("profiles/companion.yaml")).expect("profile");
    let output = harness
        .command()
        .args(["account", "status", "work", "--json"])
        .output()
        .expect("status");
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(value["profile_present"], false);
    let warnings = value["warnings"].as_array().expect("warnings");
    assert!(
        warnings.iter().any(|warning| warning
            .as_str()
            .is_some_and(|text| text.contains("no document"))),
        "{warnings:?}"
    );
}

/// A defect states what it costs and what to type next, in sentences, and
/// names no check id to do it ([ADR-0093]).
///
/// [ADR-0093]: ../docs/decisions/ADR-0093-write-every-non-machine-surface-for-a-person.md
#[test]
fn an_unbound_account_states_its_consequence_and_the_command_that_fixes_it() {
    let harness = Harness::new();
    harness.initialize_token("work", b"sk-unbound", b"sk-unbound");
    let output = harness
        .command()
        .args(["--account", "work", "account", "status", "work"])
        .output()
        .expect("status");
    let text = String::from_utf8_lossy(&output.stdout);
    let flowed = support::flowed(&text);
    assert!(
        flowed.contains(
            "It is bound to no profile, so a launch under it refuses before claude starts."
        ),
        "{text}"
    );
    assert!(
        flowed.contains("claude-session account bind work --profile <name>"),
        "{text}"
    );
    assert!(text.contains("[warn]"), "{text}");
    assert!(!text.contains("account-profile-bound"), "{text}");
}

/// The published example is the implemented one. Paths and clocks differ per
/// machine, so the sentences around them are what this pins — the same bargain
/// the `doctor` documentation test strikes.
#[test]
fn the_published_status_example_matches_the_renderer() {
    let harness = Harness::new();
    harness.initialize_token("gubasso", b"sk-doc", b"sk-doc");
    harness.write_piece("work", "{}\n");
    harness.write_profile("work", "layers:\n  - work\n");
    harness
        .assert_command()
        .args(["account", "bind", "gubasso", "--profile", "work"])
        .assert()
        .success();
    let output = harness
        .command()
        .args(["--account", "gubasso", "account", "status", "gubasso"])
        .output()
        .expect("status");
    let rendered = support::flowed(&String::from_utf8_lossy(&output.stdout));
    let document = fs::read_to_string("docs/reference/accounts.md").expect("accounts.md");
    let example = document
        .split("```text\nAccount gubasso")
        .nth(1)
        .and_then(|rest| rest.split("```").next())
        .expect("the published example");
    for sentence in [
        "This account signs in with a long-lived token this wrapper stores",
        "which is a guess from when it was recorded rather than anything the token itself says",
        "It is bound to the \"work\" profile, and that is what this run would use.",
        "This is the account a launch would use, because you named it with --account.",
    ] {
        assert!(
            support::flowed(example).contains(sentence),
            "the example dropped: {sentence}"
        );
        assert!(
            rendered.contains(sentence),
            "the renderer no longer writes: {sentence}\n{rendered}"
        );
    }
    assert!(rendered.contains("[pass]"), "{rendered}");
}

/// Returns the one session directory a launch created under an account.
///
/// Both names are derived from whatever terminal and namespace the test process
/// happens to be in, which no test can know, so the directory is found rather
/// than spelled. Two levels, because a terminal name is unique only inside the
/// namespace that issued it ([ADR-0107]).
///
/// [ADR-0107]: ../docs/decisions/ADR-0107-scope-a-terminal-to-its-namespace.md
fn only_session_dir(harness: &Harness, account: &str) -> std::path::PathBuf {
    let sessions = harness.state().join(format!("accounts/{account}/sessions"));
    let namespace = only_child(&sessions, "one launch makes one namespace directory");
    // Directories only: the witness record sits beside the directory it
    // judges ([ADR-0110]).
    //
    // [ADR-0110]: ../docs/decisions/ADR-0110-record-the-terminal-witness-at-launch.md
    let mut entries: Vec<_> = fs::read_dir(&namespace)
        .unwrap_or_else(|error| panic!("{}: {error}", namespace.display()))
        .map(|entry| entry.expect("entry").path())
        .filter(|path| path.is_dir())
        .collect();
    assert_eq!(entries.len(), 1, "one launch makes one session directory");
    entries.pop().expect("entry")
}

/// Relabels the launch's session directory as the one this process owns.
///
/// A session belongs to one running agent, so the only session directory a
/// command can inspect is the agent it is running inside ([ADR-0113]). The
/// launch under test has exited by now, so its directory is renamed to the
/// agent name this test process would carry and its record rewritten to match.
/// A wrapper spawned from here then descends from that agent and finds it.
///
/// [ADR-0113]: ../docs/decisions/ADR-0113-key-a-session-to-its-running-agent.md
fn adopt_session_as_current(harness: &Harness, account: &str) -> std::path::PathBuf {
    let made = only_session_dir(harness, account);
    let namespace = made.parent().expect("namespace").to_path_buf();
    let old_name = made
        .file_name()
        .expect("name")
        .to_string_lossy()
        .into_owned();
    let mut record: serde_json::Value = serde_json::from_slice(
        &fs::read(namespace.join(format!(".{old_name}.witness.json"))).expect("witness"),
    )
    .expect("witness json");
    let pid = std::process::id();
    let stat = fs::read_to_string(format!("/proc/{pid}/stat")).expect("stat");
    let started: u64 = stat
        .rsplit_once(')')
        .expect("comm")
        .1
        .split_whitespace()
        .nth(19)
        .expect("field 22")
        .parse()
        .expect("ticks");
    let name = format!("agent-{pid}-{started:x}");
    let adopted = namespace.join(&name);
    fs::rename(&made, &adopted).expect("adopt the session directory");
    fs::remove_file(namespace.join(format!(".{old_name}.witness.json"))).expect("old record");
    record["pid"] = serde_json::Value::from(pid);
    record["started"] = serde_json::Value::from(started);
    fs::write(
        namespace.join(format!(".{name}.witness.json")),
        serde_json::to_vec(&record).expect("record"),
    )
    .expect("adopted record");
    adopted
}

/// Returns the single entry of a directory, or fails saying what was expected.
fn only_child(directory: &std::path::Path, why: &str) -> std::path::PathBuf {
    let mut entries: Vec<_> = fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("{}: {error}", directory.display()))
        .map(|entry| entry.expect("entry").path())
        .collect();
    assert_eq!(entries.len(), 1, "{why}");
    entries.pop().expect("entry")
}

/// Slice 028 acceptance: the split itself. What a terminal owns is its own
/// child configuration and prompt history; what the account keeps is the one
/// saved login and the one projects tree, reached through a declared link.
///
/// One process is one terminal, so this proves the layout rather than the
/// two-pane case: the second terminal is the same assertion with a different
/// derived name, and nothing in the wrapper distinguishes them.
#[test]
fn a_launch_splits_state_by_terminal_and_shares_the_login_and_projects() {
    let harness = Harness::new();
    harness.initialize_login("work");
    assert!(
        harness
            .companion_profile_command()
            .args(["--account", "work"])
            .status()
            .expect("wrapper")
            .success()
    );
    let session = only_session_dir(&harness, "work");
    assert!(
        session.join(".claude.json").is_file(),
        "the terminal owns its own child configuration"
    );
    let link = session.join("projects");
    assert!(
        link.symlink_metadata().expect("projects").is_symlink(),
        "the projects tree is reached through a declared link"
    );
    assert_eq!(
        fs::read_link(&link).expect("link target"),
        harness.state().join("accounts/work/config/projects"),
        "the link points at the tree every terminal of this account shares"
    );
    assert!(
        harness
            .state()
            .join("accounts/work/config/.credentials.json")
            .exists(),
        "the saved login stays one file, outside every session directory"
    );
    assert!(
        !session.join(".credentials.json").exists(),
        "no credential is copied into a session directory"
    );
}

/// Slice 029 acceptance: the user's own assets reach the session directory,
/// and a name their tree does not hold is simply absent rather than empty.
#[test]
fn a_launch_supplies_the_assets_the_tree_holds_and_no_others() {
    let harness = Harness::new();
    harness.initialize_login("work");
    // `skills` is the fixture's baseline; `agents` is added here so the test
    // covers a supplied name and an absent one in the same run.
    fs::create_dir_all(harness.assets().join("agents")).expect("asset fixture");
    assert!(
        harness
            .companion_profile_command()
            .args(["--account", "work"])
            .status()
            .expect("wrapper")
            .success()
    );
    let session = only_session_dir(&harness, "work");
    for name in ["skills", "agents"] {
        let link = session.join(name);
        assert!(
            link.symlink_metadata()
                .unwrap_or_else(|_| panic!("{name} was not supplied"))
                .is_symlink(),
            "{name} reaches the session directory as a declared link"
        );
        assert_eq!(
            fs::read_link(&link).expect("link target"),
            harness.assets().join(name)
        );
    }
    for name in ["commands", "rules", "workflows", "themes", "CLAUDE.md"] {
        assert!(
            session.join(name).symlink_metadata().is_err(),
            "{name} is absent from the tree, so nothing is created for it"
        );
    }
    assert!(
        session.join("plugins").symlink_metadata().is_err(),
        "the plugin tree is never linked, and no seed means nothing is created"
    );
}

/// Slice 038 acceptance: the seed's plugin state reaches the session directory,
/// as a copy the child may write and never as a link into a read-only tree.
///
/// A state file the seed does not hold is skipped rather than materialised
/// empty, for the reason an absent asset is: an empty file would assert
/// something the user did not.
#[test]
fn a_launch_copies_the_plugin_state_the_seed_holds_and_no_more() {
    let harness = Harness::new();
    harness.initialize_login("work");
    let seed = harness.write_plugin_seed(&["known_marketplaces.json"]);
    assert!(
        harness
            .companion_profile_command()
            .args(["--account", "work"])
            .status()
            .expect("wrapper")
            .success()
    );
    let session = only_session_dir(&harness, "work");
    let copied = session.join("plugins/known_marketplaces.json");
    let facts = copied.symlink_metadata().expect("the state was not copied");
    assert!(
        facts.is_file(),
        "the copy is a real file the child may write"
    );
    assert_eq!(
        fs::read(&copied).expect("copied state"),
        fs::read(seed.join("known_marketplaces.json")).expect("seed state"),
        "the wrapper copies the bytes it found and changes none of them"
    );
    assert_eq!(facts.permissions().mode() & 0o7777, 0o600);
    assert!(
        session
            .join("plugins/installed_plugins.json")
            .symlink_metadata()
            .is_err(),
        "a state file the seed does not hold is not materialised empty"
    );
}

/// Slice 038 acceptance: the seed is read and never written, so one tree can
/// serve every session without any of them changing it for the others.
#[test]
fn a_launch_never_writes_into_the_plugin_seed() {
    let harness = Harness::new();
    harness.initialize_login("work");
    let seed = harness.write_plugin_seed(&["known_marketplaces.json", "installed_plugins.json"]);
    let before = seed_fingerprint(&seed);
    assert!(
        harness
            .companion_profile_command()
            .args(["--account", "work"])
            .status()
            .expect("wrapper")
            .success()
    );
    assert_eq!(before, seed_fingerprint(&seed), "the seed was modified");
}

/// Every path below the seed with the bytes it holds, so a write anywhere in
/// the tree — a new file included — shows up as a difference.
fn seed_fingerprint(seed: &std::path::Path) -> Vec<(std::path::PathBuf, Vec<u8>)> {
    let mut found = Vec::new();
    let mut pending = vec![seed.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory).expect("seed tree") {
            let path = entry.expect("seed entry").path();
            if path.is_dir() {
                pending.push(path);
            } else {
                let bytes = fs::read(&path).expect("seed file");
                found.push((path, bytes));
            }
        }
    }
    found.sort();
    found
}

/// Slice 028 acceptance: the declared-links check covers every link the wrapper
/// declared, which includes the assets a launch supplies.
///
/// Without this the report and the launch disagree: `doctor` passes while the
/// next launch refuses the same tree.
#[test]
fn a_repointed_asset_link_fails_the_declared_links_check() {
    let harness = Harness::new();
    harness.initialize_login("work");
    assert!(
        harness
            .companion_profile_command()
            .args(["--account", "work"])
            .status()
            .expect("wrapper")
            .success()
    );
    let session = adopt_session_as_current(&harness, "work");
    let link = session.join("skills");
    fs::remove_file(&link).expect("declared link");
    std::os::unix::fs::symlink(harness.root().join("elsewhere"), &link).expect("repointed link");
    // The source goes too. A launch validates the seat whether or not the tree
    // still holds the asset, so a report that looked only at held names would
    // pass here while the next launch refused.
    fs::remove_dir(harness.assets().join("skills")).expect("asset fixture");
    let output = harness
        .assert_command()
        .args(["--profile", "companion", "--account", "work", "doctor"])
        .output()
        .expect("doctor");
    let stdout = support::flowed(&String::from_utf8_lossy(&output.stdout));
    assert!(
        stdout.contains("Shared links in the session directory") && stdout.contains("[fail]"),
        "the repointed asset link must fail storage-declared-links: {stdout}"
    );
}

/// Slice 028 acceptance: the launch that creates a session directory answers
/// the question that directory's newness makes the child ask.
#[test]
fn a_launch_records_the_first_run_setup_as_done() {
    let harness = Harness::new();
    harness.initialize_login("work");
    assert!(
        harness
            .companion_profile_command()
            .args(["--account", "work"])
            .status()
            .expect("wrapper")
            .success()
    );
    let path = only_session_dir(&harness, "work").join(".claude.json");
    let document: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).expect("child configuration")).expect("json");
    assert_eq!(document["hasCompletedOnboarding"], true);
    assert_eq!(mode(&path), 0o600);
}

/// Slice 028 acceptance: while the trust key is enabled, a launch records the
/// working directory as trusted in the session directory it created.
///
/// Enabled is the default, so this is also what the gate's default answers for.
/// The launch directory is the harness root, which is what every wrapper
/// command in these tests runs in.
#[test]
fn a_launch_records_the_working_directory_as_trusted() {
    let harness = Harness::new();
    harness.initialize_login("work");
    assert!(
        harness
            .companion_profile_command()
            .args(["--account", "work"])
            .status()
            .expect("wrapper")
            .success()
    );
    let path = only_session_dir(&harness, "work").join(".claude.json");
    let document: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).expect("child configuration")).expect("json");
    let workspace = &document["projects"][harness.root().to_str().expect("utf-8 fixture path")];
    // Both keys, because the child asks two questions about a new workspace and
    // answering one leaves the other prompt standing.
    assert_eq!(workspace["hasTrustDialogAccepted"], true, "{document}");
    assert_eq!(
        workspace["hasCompletedProjectOnboarding"], true,
        "{document}"
    );
}

/// The gate is a configuration key, so turning it off has to reach the launch.
///
/// Both layers the key accepts are exercised, because a launch that read only
/// one of them would leave the other silently inert.
#[test]
fn a_disabled_trust_gate_records_no_workspace() {
    for layer in ["file", "environment"] {
        let harness = Harness::new();
        harness.initialize_login("work");
        if layer == "file" {
            fs::create_dir_all(harness.config_base()).expect("config fixture");
            fs::write(
                harness.config_base().join("config.toml"),
                "auto_trust_cwd = false\n",
            )
            .expect("config fixture");
        }
        let mut command = harness.companion_profile_command();
        command.args(["--account", "work"]);
        if layer == "environment" {
            command.env("CLAUDE_SESSION_AUTO_TRUST_CWD", "false");
        }
        assert!(command.status().expect("wrapper").success());
        let path = only_session_dir(&harness, "work").join(".claude.json");
        let document: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).expect("child configuration")).expect("json");
        // The seed the launch owns unconditionally is still there; only the
        // workspace answer is withheld.
        assert_eq!(document["hasCompletedOnboarding"], true, "{layer}");
        assert!(
            document.get("projects").is_none(),
            "{layer} left a workspace record the gate refuses: {document}"
        );
    }
}

/// Slice 038 acceptance: while the key is enabled, a launch records the child's
/// plugin recommendation as answered in the session directory it created, and
/// while it is unset the child keeps its own behaviour.
///
/// Both layers the key accepts are exercised, for the reason the trust gate's
/// test gives: a launch that read only one would leave the other inert.
#[test]
fn a_launch_answers_the_plugin_recommendation_only_when_asked() {
    for layer in ["unset", "file", "environment"] {
        let harness = Harness::new();
        harness.initialize_login("work");
        if layer == "file" {
            fs::create_dir_all(harness.config_base()).expect("config fixture");
            fs::write(
                harness.config_base().join("config.toml"),
                "suppress_lsp_recommendations = true\n",
            )
            .expect("config fixture");
        }
        let mut command = harness.companion_profile_command();
        command.args(["--account", "work"]);
        if layer == "environment" {
            command.env("CLAUDE_SESSION_SUPPRESS_LSP_RECOMMENDATIONS", "true");
        }
        assert!(command.status().expect("wrapper").success());
        let path = only_session_dir(&harness, "work").join(".claude.json");
        let document: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).expect("child configuration")).expect("json");
        if layer == "unset" {
            assert!(
                document.get("lspRecommendationDisabled").is_none(),
                "an unset key must leave the child asking: {document}"
            );
        } else {
            assert_eq!(document["lspRecommendationDisabled"], true, "{layer}");
        }
        // The seed the launch owns unconditionally is there either way, so a
        // failure above is about this key rather than about the write.
        assert_eq!(document["hasCompletedOnboarding"], true, "{layer}");
    }
}

/// The half of "ready" a reader cannot check for themselves is the half the
/// report has to state.
///
/// The profile is written first, because the whole-launch claim is only true of
/// an account whose profile resolves. The account that has no document is the
/// test below.
#[test]
fn a_login_report_says_the_launch_reaches_the_prompt() {
    let harness = Harness::new();
    harness.initialize_companion_profile();
    let output = harness
        .assert_command()
        .args([
            "account",
            "login",
            "work",
            "--profile",
            "companion",
            "--token",
            "--stdin",
        ])
        .write_stdin("sk-report\n")
        .output()
        .expect("token login");
    let stdout = support::flowed(&String::from_utf8_lossy(&output.stdout));
    assert!(
        stdout.contains("A launch under it goes straight to claude's prompt"),
        "{stdout}"
    );
}

/// A report that warns the launch refuses must not also promise it reaches the
/// prompt.
///
/// The first-run row is still a pass, because it states what a launch does
/// rather than something this login wrote: since ADR-0105 the seed happens when
/// a launch creates the terminal's session directory. What narrows is the claim
/// built on top of it: the profile row above has just said this launch stops
/// before the child starts, and two rows disagreeing leave the reader with no
/// report at all.
#[test]
fn a_login_report_without_a_profile_document_claims_only_what_it_recorded() {
    let harness = Harness::new();
    let output = harness
        .assert_command()
        .args([
            "account",
            "login",
            "work",
            "--profile",
            "companion",
            "--token",
            "--stdin",
        ])
        .write_stdin("sk-report\n")
        .output()
        .expect("token login");
    let stdout = support::flowed(&String::from_utf8_lossy(&output.stdout));
    assert!(
        stdout.contains("that profile has no document yet"),
        "{stdout}"
    );
    assert!(
        !stdout.contains("goes straight to claude's prompt"),
        "the report must not promise a launch the profile row just refused: {stdout}"
    );
    assert!(
        stdout.contains("a launch records claude's first-run setup as done before starting it"),
        "the report must promise the launch's own write, not one this login made: {stdout}"
    );
}

/// An account created before this behaviour existed still launches, and the
/// reader hears what they are about to meet before the exec rather than after.
#[test]
fn a_launch_into_a_fresh_session_answers_onboarding_rather_than_warning() {
    let harness = Harness::new();
    harness.initialize_login("work");
    let output = harness
        .companion_profile_command()
        .args(["--account", "work", "run"])
        .output()
        .expect("launch");
    assert!(output.status.success());
    let stderr = support::flowed(&String::from_utf8_lossy(&output.stderr));
    // The warning belonged to a launch that could only read this file. Since
    // the launch writes it on the way past, warning would name a state nothing
    // can be in by the time the child runs.
    assert!(
        !stderr.contains("first-run setup is done"),
        "a launch that seeds the key does not warn about it: {stderr}"
    );
    let document: serde_json::Value = serde_json::from_slice(
        &fs::read(only_session_dir(&harness, "work").join(".claude.json")).expect("seeded"),
    )
    .expect("json");
    assert_eq!(document["hasCompletedOnboarding"], true);
}

/// Slice 025 acceptance: the plan the command line declared reaches the
/// account's metadata, beside the token it describes.
#[test]
fn a_token_login_records_the_declared_plan() {
    let harness = Harness::new();
    let output = harness
        .assert_command()
        .args([
            "account",
            "login",
            "work",
            "--profile",
            "companion",
            "--token",
            "--stdin",
            "--plan",
            "Max",
        ])
        .write_stdin("sk-declared\n")
        .output()
        .expect("token login");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metadata: serde_json::Value = serde_json::from_slice(
        &fs::read(harness.state().join("accounts/work/auth-mode.json")).expect("metadata file"),
    )
    .expect("metadata");
    assert_eq!(
        metadata["plan"], "max",
        "the declaration is recorded, lowercased"
    );
}

/// Slice 025 acceptance: a terminal login with no `--plan` asks, and the answer
/// is recorded. The offer is numbered, so `1` is the first plan it lists.
#[test]
fn a_token_login_asks_for_the_plan_and_records_the_answer() {
    let harness = Harness::new();
    let output = interactive_token_login(&harness, "1\n");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let terminal = support::flowed(&String::from_utf8_lossy(&output.stdout));
    assert!(
        terminal.contains("Which plan does this token belong to?"),
        "{terminal}"
    );
    let metadata: serde_json::Value = serde_json::from_slice(
        &fs::read(harness.state().join("accounts/work/auth-mode.json")).expect("metadata file"),
    )
    .expect("metadata");
    assert_eq!(metadata["plan"], "max", "the numbered answer is resolved");
}

/// Slice 025 acceptance: declining the question is not refusing the login. The
/// credential was already proven by the time it was asked.
#[test]
fn a_declined_plan_prompt_still_completes_the_login() {
    let harness = Harness::new();
    let output = interactive_token_login(&harness, "\n");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let metadata: serde_json::Value = serde_json::from_slice(
        &fs::read(harness.state().join("accounts/work/auth-mode.json")).expect("metadata file"),
    )
    .expect("metadata");
    assert_eq!(metadata["mode"], "token", "the login committed");
    assert!(
        metadata.get("plan").is_none(),
        "and declared no plan: {metadata}"
    );
}

/// Runs an interactive token login, answering the token prompt and then the
/// plan prompt.
///
/// Each answer waits for the prompt that asks for it rather than for a clock.
/// The secret read flushes type-ahead entered before it and drains anything
/// queued behind its first newline, so an answer written early is either
/// discarded or swallowed as part of the token — and a sleep long enough to
/// avoid that on one machine is a race on another.
fn interactive_token_login(harness: &Harness, answer: &str) -> std::process::Output {
    use std::io::{Read as _, Write as _};
    let mut child = harness
        .terminal_command("account login work --profile companion --token")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("terminal login");
    let mut terminal = child.stdout.take().expect("terminal output");
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let collected = std::sync::Arc::clone(&seen);
    let reader = std::thread::spawn(move || {
        let mut byte = [0_u8; 1];
        while terminal.read(&mut byte).unwrap_or(0) == 1 {
            collected.lock().expect("terminal buffer").push(byte[0]);
        }
    });
    let mut stdin = child.stdin.take().expect("terminal input");
    for (prompt, reply) in [("Paste the token", "sk-asked\n"), ("Which plan", answer)] {
        await_prompt(&seen, prompt);
        stdin.write_all(reply.as_bytes()).expect("terminal reply");
        stdin.flush().expect("terminal reply");
    }
    let status = child.wait().expect("terminal login");
    drop(stdin);
    reader.join().expect("terminal reader");
    let stdout = seen.lock().expect("terminal buffer").clone();
    std::process::Output {
        status,
        stdout,
        stderr: Vec::new(),
    }
}

/// Blocks until the pty has written `prompt`, or the harness deadline passes.
fn await_prompt(seen: &std::sync::Mutex<Vec<u8>>, prompt: &str) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    while std::time::Instant::now() < deadline {
        {
            let buffer = seen.lock().expect("terminal buffer");
            if String::from_utf8_lossy(&buffer).contains(prompt) {
                return;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let buffer = seen.lock().expect("terminal buffer");
    panic!(
        "{prompt:?} never appeared; the terminal said: {}",
        String::from_utf8_lossy(&buffer)
    );
}

/// Slice 025 acceptance: a declaration outside the shape rule is a usage error,
/// so it lands before a credential is read and before an account exists.
#[test]
fn a_malformed_plan_is_refused_before_ingest() {
    let harness = Harness::new();
    let output = harness
        .assert_command()
        .args([
            "account",
            "login",
            "work",
            "--profile",
            "companion",
            "--token",
            "--stdin",
            "--plan",
            "max plan",
        ])
        .write_stdin("sk-never-read\n")
        .output()
        .expect("token login");
    assert_eq!(output.status.code(), Some(64));
    assert!(
        !harness.state().join("accounts/work").exists(),
        "a usage error precedes every side effect"
    );
}

/// Slice 025 acceptance: the recorded plan reaches the child in the variable it
/// reads a plan from.
#[test]
fn a_token_launch_injects_the_recorded_plan() {
    let harness = Harness::new();
    harness.initialize_token("work", b"sk-planned", b"sk-planned");
    let output = harness
        .companion_profile_command()
        .args(["--account", "work", "run"])
        .output()
        .expect("launch");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let environ = read_nul(&harness.record_dir().join("environ"));
    let index = environ
        .iter()
        .position(|value| value == b"CLAUDE_CODE_SUBSCRIPTION_TYPE")
        .expect("the declared plan is injected");
    assert_eq!(environ[index + 1], b"max");
}

/// Slice 025 acceptance: an account that declared none is told about and
/// launched anyway, with no variable invented on its behalf.
#[test]
fn a_launch_without_a_declared_plan_warns_and_still_execs() {
    let harness = Harness::new();
    harness.initialize_token("work", b"sk-unplanned", b"sk-unplanned");
    let metadata = harness.state().join("accounts/work/auth-mode.json");
    let recorded = fs::read_to_string(&metadata).expect("metadata");
    fs::write(&metadata, recorded.replace(",\"plan\":\"max\"", ""))
        .expect("predate the declaration");
    // Seeded ambient, because inheriting it is the way the child would learn a
    // plan this account never declared: the scrub keeps every child-owned name,
    // so only token mode's own removal stands between the warning and a
    // contradicting variable.
    let output = harness
        .companion_profile_command()
        .args(["--account", "work", "run"])
        .env("CLAUDE_CODE_SUBSCRIPTION_TYPE", "pro")
        .output()
        .expect("launch");
    assert!(
        output.status.success(),
        "the launch is never refused over it"
    );
    let stderr = support::flowed(&String::from_utf8_lossy(&output.stderr));
    assert!(
        stderr.contains("this account declared no subscription plan"),
        "{stderr}"
    );
    let environ = read_nul(&harness.record_dir().join("environ"));
    assert!(
        !environ
            .iter()
            .any(|value| value == b"CLAUDE_CODE_SUBSCRIPTION_TYPE"),
        "an undeclared plan sets nothing, and inherits nothing"
    );
}

/// Slice 025 acceptance: the report names the plan, because the whole point of
/// declaring one is what the child will then say.
#[test]
fn a_login_report_names_the_declared_plan() {
    let harness = Harness::new();
    let output = harness
        .assert_command()
        .args([
            "account",
            "login",
            "work",
            "--profile",
            "companion",
            "--token",
            "--stdin",
            "--plan",
            "team",
        ])
        .write_stdin("sk-report\n")
        .output()
        .expect("token login");
    let stdout = support::flowed(&String::from_utf8_lossy(&output.stdout));
    assert!(stdout.contains("It declares the team plan"), "{stdout}");
}

/// Reads one variable out of the recording child's environment dump.
fn recorded_env(harness: &Harness, key: &str) -> Option<Vec<u8>> {
    let environ = read_nul(&harness.record_dir().join("environ"));
    environ
        .iter()
        .position(|value| value == key.as_bytes())
        .and_then(|index| environ.get(index + 1).cloned())
}

/// Runs a refresh-token login with the secret on standard input.
fn refresh_login(harness: &Harness, secret: &str, extra: &[&str]) -> std::process::Output {
    let mut command = harness.assert_command();
    command.args([
        "account",
        "login",
        "work",
        "--profile",
        "companion",
        "--refresh-token",
        "--stdin",
    ]);
    command.args(extra);
    command
        .write_stdin(format!("{secret}\n"))
        .output()
        .expect("refresh login")
}

/// Slice 026 acceptance: the exchange ends in the child's own saved login, so
/// the account records the mode a browser login records and nothing more.
#[test]
fn a_refresh_login_records_a_saved_login() {
    let harness = Harness::new();
    let mut command = harness.assert_command();
    command
        .args([
            "account",
            "login",
            "work",
            "--profile",
            "companion",
            "--refresh-token",
            "--stdin",
        ])
        .env("CS_TEST_CREATE_CREDENTIAL", "1");
    let output = command
        .write_stdin("sk-ant-ort01-recorded\n")
        .output()
        .expect("refresh login");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let account = harness.state().join("accounts/work");
    let metadata: serde_json::Value =
        serde_json::from_slice(&fs::read(account.join("auth-mode.json")).expect("metadata"))
            .expect("json");
    assert_eq!(metadata["mode"], "login");
    assert!(
        metadata.get("fingerprint").is_none(),
        "there is no wrapper-owned secret to describe"
    );
    assert_eq!(
        fs::read(account.join("config/.credentials.json")).expect("credential"),
        b"child-owned-login-bytes"
    );
}

/// Slice 026 acceptance: the secret reaches the child the one way the ingest
/// rule permits it to leave this process, so it cannot appear in a listing.
#[test]
fn a_refresh_login_carries_the_secret_in_the_environment_only() {
    let harness = Harness::new();
    let mut command = harness.assert_command();
    command
        .args([
            "account",
            "login",
            "work",
            "--profile",
            "companion",
            "--refresh-token",
            "--stdin",
        ])
        .env("CS_TEST_CREATE_CREDENTIAL", "1");
    let output = command
        .write_stdin("sk-ant-ort01-environment\n")
        .output()
        .expect("refresh login");
    assert!(output.status.success());
    let argv = read_nul(&harness.record_dir().join("argv"));
    assert_eq!(&argv[1..], &[b"auth".to_vec(), b"login".to_vec()]);
    assert!(
        !argv
            .iter()
            .any(|value| value == b"sk-ant-ort01-environment"),
        "a credential never enters through argv"
    );
    assert_eq!(
        recorded_env(&harness, "CLAUDE_CODE_OAUTH_REFRESH_TOKEN").as_deref(),
        Some(b"sk-ant-ort01-environment".as_slice())
    );
}

/// The ingest accepts every non-control byte on purpose, so that a change in
/// the provider's token format cannot make it refuse a working credential. A
/// conversion through a Rust string on the way to the child would undo that
/// silently: the secret would be accepted here and substituted before it left,
/// and the exchange would fail for a reason nothing local could name.
#[test]
fn a_refresh_token_reaches_the_child_byte_for_byte() {
    let harness = Harness::new();
    // Accepted by the ingest — no byte below 0x20 and no 0x7f — and not valid
    // UTF-8, which is the pair that makes a lossy conversion observable.
    let secret: Vec<u8> = b"sk-ant-ort01-\xff\xfe-raw".to_vec();
    let mut command = harness.assert_command();
    command
        .args([
            "account",
            "login",
            "work",
            "--profile",
            "companion",
            "--refresh-token",
            "--stdin",
        ])
        .env("CS_TEST_CREATE_CREDENTIAL", "1");
    let mut stdin = secret.clone();
    stdin.push(b'\n');
    let output = command.write_stdin(stdin).output().expect("refresh login");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        recorded_env(&harness, "CLAUDE_CODE_OAUTH_REFRESH_TOKEN").as_deref(),
        Some(secret.as_slice()),
        "the child receives the bytes the ingest accepted"
    );
}

/// Slice 026 acceptance: the common case is a token a claude.ai login issued,
/// and the wrapper types that set rather than making the user know it.
#[test]
fn a_refresh_login_without_scopes_uses_the_set_a_login_is_issued() {
    let harness = Harness::new();
    let mut command = harness.assert_command();
    command
        .args([
            "account",
            "login",
            "work",
            "--profile",
            "companion",
            "--refresh-token",
            "--stdin",
        ])
        .env("CS_TEST_CREATE_CREDENTIAL", "1");
    let output = command
        .write_stdin("sk-ant-ort01-default\n")
        .output()
        .expect("refresh login");
    assert!(output.status.success());
    // Spelled out rather than imported, so a change to the default has to be
    // made here too and cannot pass by agreeing with itself.
    let expected = [
        "org:create_api_key",
        "user:profile",
        "user:inference",
        "user:sessions:claude_code",
        "user:mcp_servers",
        "user:file_upload",
    ]
    .join(" ");
    assert_eq!(
        recorded_env(&harness, "CLAUDE_CODE_OAUTH_SCOPES").as_deref(),
        Some(expected.as_bytes())
    );
}

/// Slice 026 acceptance: a grant issued with a different set has to be able to
/// say so, or the exchange fails on scopes the wrapper invented.
#[test]
fn a_declared_scope_set_replaces_the_default() {
    let harness = Harness::new();
    let mut command = harness.assert_command();
    command
        .args([
            "account",
            "login",
            "work",
            "--profile",
            "companion",
            "--refresh-token",
            "--stdin",
            "--scopes",
            "  user:inference   user:profile  ",
        ])
        .env("CS_TEST_CREATE_CREDENTIAL", "1");
    let output = command
        .write_stdin("sk-ant-ort01-scoped\n")
        .output()
        .expect("refresh login");
    assert!(output.status.success());
    assert_eq!(
        recorded_env(&harness, "CLAUDE_CODE_OAUTH_SCOPES").as_deref(),
        Some(b"user:inference user:profile".as_slice()),
        "the separators are normalized to the ones the child splits on"
    );
}

/// Slice 026 acceptance: a malformed value is a usage error, and a usage error
/// costs nothing — no directory, no child, no prompt.
#[test]
fn a_malformed_scope_set_is_refused_before_the_child_runs() {
    let harness = Harness::new();
    let output = refresh_login(&harness, "sk-ant-ort01-unused", &["--scopes", "bad\"scope"]);
    assert_eq!(
        output.status.code(),
        Some(64),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!harness.record_dir().join("argv").exists());
    assert!(!harness.state().join("accounts/work").exists());
}

/// Slice 026 acceptance: the absence of a terminal is the whole reason this
/// path exists, so it must not be what stops it.
#[test]
fn a_refresh_login_needs_no_terminal() {
    use std::io::Write as _;
    let harness = Harness::new();
    let mut command = harness.detached_command(&[
        "account",
        "login",
        "work",
        "--profile",
        "companion",
        "--refresh-token",
        "--stdin",
    ]);
    command
        .env("CS_TEST_CREATE_CREDENTIAL", "1")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = command.spawn().expect("detached refresh login");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(b"sk-ant-ort01-headless\n")
        .expect("secret");
    let output = child.wait_with_output().expect("detached refresh login");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metadata: serde_json::Value = serde_json::from_slice(
        &fs::read(harness.state().join("accounts/work/auth-mode.json")).expect("metadata"),
    )
    .expect("json");
    assert_eq!(metadata["mode"], "login");
}

/// Slice 026 acceptance: the credential the child leaves is the only evidence
/// the exchange worked, so a child that leaves none has not logged anything in.
#[test]
fn a_refresh_login_that_leaves_no_saved_login_removes_the_account() {
    let harness = Harness::new();
    let output = refresh_login(&harness, "sk-ant-ort01-empty", &[]);
    assert_eq!(
        output.status.code(),
        Some(77),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!harness.state().join("accounts/work").exists());
}

/// Slice 026 acceptance: the refresh token is spent, not kept. It may be
/// rotated by the exchange, so a copy on disk could be dead with nothing local
/// able to tell.
#[test]
fn a_refresh_token_is_never_written_under_the_account() {
    let harness = Harness::new();
    let mut command = harness.assert_command();
    command
        .args([
            "account",
            "login",
            "work",
            "--profile",
            "companion",
            "--refresh-token",
            "--stdin",
        ])
        .env("CS_TEST_CREATE_CREDENTIAL", "1");
    let output = command
        .write_stdin("sk-ant-ort01-unstored\n")
        .output()
        .expect("refresh login");
    assert!(output.status.success());
    let mut examined = 0_usize;
    let mut pending = vec![harness.state().join("accounts/work")];
    while let Some(path) = pending.pop() {
        for entry in fs::read_dir(&path).expect("account tree") {
            let entry = entry.expect("entry").path();
            if entry.is_dir() {
                pending.push(entry);
                continue;
            }
            let bytes = fs::read(&entry).expect("account file");
            examined += 1;
            assert!(
                !bytes
                    .windows(b"sk-ant-ort01-unstored".len())
                    .any(|window| window == b"sk-ant-ort01-unstored"),
                "{} holds the refresh token",
                entry.display()
            );
        }
    }
    assert!(examined > 0, "the walk has to have read something");
}

/// Slice 026 acceptance: the two selectors are answers to the same question, so
/// the parser refuses both rather than the wrapper picking one.
#[test]
fn a_token_and_a_refresh_token_together_are_refused() {
    let harness = Harness::new();
    let output = harness
        .assert_command()
        .args([
            "account",
            "login",
            "work",
            "--profile",
            "companion",
            "--token",
            "--refresh-token",
            "--stdin",
        ])
        .write_stdin("sk-unused\n")
        .output()
        .expect("conflicting login");
    assert_eq!(
        output.status.code(),
        Some(64),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!harness.record_dir().join("argv").exists());
    assert!(!harness.state().join("accounts/work").exists());
}

/// Stores a token under `work`, so a later login has something to supersede.
fn token_account(harness: &Harness) {
    let output = harness
        .assert_command()
        .args([
            "account",
            "login",
            "work",
            "--profile",
            "companion",
            "--token",
            "--stdin",
        ])
        .write_stdin("sk-superseded-token\n")
        .output()
        .expect("token login");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Leaves `work` holding a child-owned saved login, without needing a terminal.
fn saved_login_account(harness: &Harness) {
    let output = harness
        .assert_command()
        .args([
            "account",
            "login",
            "work",
            "--profile",
            "companion",
            "--refresh-token",
            "--stdin",
        ])
        .env("CS_TEST_CREATE_CREDENTIAL", "1")
        .write_stdin("sk-ant-ort01-seed\n")
        .output()
        .expect("refresh login");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Slice 027 acceptance: recording a saved login takes the stored token with
/// it, and leaves the credential it just recorded alone.
#[test]
fn a_native_login_retires_the_token_it_supersedes() {
    let harness = Harness::new();
    token_account(&harness);
    let account = harness.state().join("accounts/work");
    assert!(account.join("oauth-token").is_file(), "fixture");
    let output = harness
        .terminal_command("account login work --profile companion")
        .env("CS_TEST_CREATE_CREDENTIAL", "1")
        .output()
        .expect("native login");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !account.join("oauth-token").exists(),
        "the superseded token survived the login that replaced it"
    );
    assert_eq!(
        fs::read(account.join("config/.credentials.json")).expect("credential"),
        b"child-owned-login-bytes",
        "the credential this login recorded must survive it"
    );
    let metadata: serde_json::Value =
        serde_json::from_slice(&fs::read(account.join("auth-mode.json")).expect("metadata"))
            .expect("json");
    assert_eq!(metadata["mode"], "login");
}

/// Slice 027 acceptance: the retirement rides the commit rather than the entry
/// point, so the terminal-less login gets it on the same terms.
#[test]
fn a_refresh_login_retires_the_token_it_supersedes() {
    let harness = Harness::new();
    token_account(&harness);
    let account = harness.state().join("accounts/work");
    let output = harness
        .assert_command()
        .args([
            "account",
            "login",
            "work",
            "--profile",
            "companion",
            "--refresh-token",
            "--stdin",
        ])
        .env("CS_TEST_CREATE_CREDENTIAL", "1")
        .write_stdin("sk-ant-ort01-replacement\n")
        .output()
        .expect("refresh login");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!account.join("oauth-token").exists());
    assert!(account.join("config/.credentials.json").is_file());
}

/// Slice 027 acceptance: the other direction, where what is abandoned is the
/// child's own credential rather than the wrapper's.
#[test]
fn a_token_login_retires_the_saved_login_it_supersedes() {
    let harness = Harness::new();
    saved_login_account(&harness);
    let account = harness.state().join("accounts/work");
    assert!(
        account.join("config/.credentials.json").is_file(),
        "fixture"
    );
    let output = harness
        .assert_command()
        .args([
            "account",
            "login",
            "work",
            "--profile",
            "companion",
            "--token",
            "--stdin",
        ])
        .write_stdin("sk-replacing-token\n")
        .output()
        .expect("token login");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !account.join("config/.credentials.json").exists(),
        "the superseded saved login survived the token that replaced it"
    );
    assert!(
        account.join("config").is_dir(),
        "only the credential is retired, not the directory the child owns"
    );
    assert_eq!(
        fs::read(account.join("oauth-token")).expect("token file"),
        b"sk-replacing-token"
    );
}

/// Slice 027 acceptance: an account that never held the other artifact is the
/// common case, and it must not read as a failed retirement.
#[test]
fn a_login_that_supersedes_nothing_succeeds_and_says_nothing() {
    let harness = Harness::new();
    let output = harness
        .assert_command()
        .args([
            "account",
            "login",
            "work",
            "--profile",
            "companion",
            "--token",
            "--stdin",
            "--json",
        ])
        .write_stdin("sk-first-token\n")
        .output()
        .expect("token login");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(report["mode"], "token");
    assert!(
        report.get("retired_superseded_credential").is_none(),
        "a login with nothing to retire must not report one: {report}"
    );
}

/// Slice 027 acceptance: a retirement is irreversible, so the report names it.
#[test]
fn a_login_that_retires_a_credential_reports_it() {
    let harness = Harness::new();
    harness.initialize_companion_profile();
    token_account(&harness);
    let output = harness
        .terminal_command("account login work --profile companion")
        .env("CS_TEST_CREATE_CREDENTIAL", "1")
        .output()
        .expect("native login");
    let stdout = support::flowed(&String::from_utf8_lossy(&output.stdout));
    assert!(
        stdout.contains("The token this account had stored is gone"),
        "{stdout}"
    );
    let json = harness
        .assert_command()
        .args([
            "account",
            "login",
            "work",
            "--profile",
            "companion",
            "--token",
            "--stdin",
            "--json",
        ])
        .write_stdin("sk-back-to-token\n")
        .output()
        .expect("token login");
    let report: serde_json::Value = serde_json::from_slice(&json.stdout).expect("json");
    assert_eq!(report["retired_superseded_credential"], true);
}

/// Slice 027 acceptance: the credential is durable before the retirement runs,
/// so a retirement that cannot complete reports rather than undoes.
#[test]
fn a_retirement_that_cannot_complete_keeps_the_credential_it_committed() {
    let harness = Harness::new();
    token_account(&harness);
    let account = harness.state().join("accounts/work");
    // A directory where the token file was: `remove_file` refuses it, which is
    // the one way to reach the failure branch without an unwritable parent.
    fs::remove_file(account.join("oauth-token")).expect("fixture");
    fs::create_dir(account.join("oauth-token")).expect("fixture");
    let output = harness
        .terminal_command("account login work --profile companion")
        .env("CS_TEST_CREATE_CREDENTIAL", "1")
        .output()
        .expect("native login");
    // The pseudo-terminal merges both streams, so the diagnostic is read out of
    // one buffer rather than assumed to be on standard error.
    let reported = support::flowed(&format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    ));
    assert!(!output.status.success(), "{reported}");
    assert!(
        reported.contains("superseded credential could not be retired"),
        "{reported}"
    );
    assert!(
        reported.contains("oauth-token"),
        "the surviving path is what the reader acts on: {reported}"
    );
    assert_eq!(
        fs::read(account.join("config/.credentials.json")).expect("credential"),
        b"child-owned-login-bytes",
        "the login is reported, not undone"
    );
    let metadata: serde_json::Value =
        serde_json::from_slice(&fs::read(account.join("auth-mode.json")).expect("metadata"))
            .expect("json");
    assert_eq!(metadata["mode"], "login");
}
