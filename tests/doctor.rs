#![allow(clippy::expect_used, clippy::pedantic, clippy::nursery)]

mod support;

use support::Harness;

const IDS: [&str; 16] = [
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
    "account-registry-readable",
    "credentials-usable",
    "settings-profile-valid",
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
    assert!(text.contains("wrapper status=pass total=16"));
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
    assert_eq!(checks.len(), 16);
    for (row, id) in checks.iter().zip(IDS) {
        assert_eq!(row["id"], id);
    }
    // Three levels, each stating its own status and code.
    assert_eq!(value["wrapper"]["summary"]["total"], 16);
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
    assert_eq!(text.lines().count(), 16);
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
    assert_eq!(rows.len(), 16);
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
    assert_eq!(skipped, 10);
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
        "settings-profile-valid"
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
    assert_eq!(value["wrapper"]["checks"][14]["status"], "skipped");
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
        16
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

#[test]
fn doctor_evaluates_selected_account_usability_locally() {
    let harness = Harness::new();
    harness.initialize_login("work");
    let output = healthy(&harness)
        .args(["--account", "work", "doctor", "--json"])
        .output()
        .expect("doctor");
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(value["wrapper"]["checks"][13]["status"], "pass");
    assert_eq!(value["wrapper"]["checks"][14]["status"], "pass");
    assert_eq!(
        std::fs::read(
            harness
                .state()
                .join("accounts/work/config/.credentials.json")
        )
        .expect("fixture"),
        b"child-owned-fixture"
    );

    std::fs::remove_file(
        harness
            .state()
            .join("accounts/work/config/.credentials.json"),
    )
    .expect("remove fixture");
    let output = healthy(&harness)
        .args(["--account", "work", "doctor", "--json"])
        .output()
        .expect("doctor");
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(value["wrapper"]["checks"][14]["status"], "warn");
    assert_eq!(value["wrapper"]["checks"][14]["kind"], "Auth");
}

/// Reads a checked-in repository document. A missing input is a failure that
/// names the path, never a skip: a gate that can quietly decline to run is not
/// a gate.
fn document(relative: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{relative} is required by this gate: {error}"))
}

/// The body of one `## ` section, up to the next one.
fn section<'a>(text: &'a str, heading: &'a str) -> &'a str {
    let start = text.find(heading).unwrap_or_else(|| panic!("{heading}"));
    let rest = &text[start + heading.len()..];
    rest.find("\n## ").map_or(rest, |end| &rest[..end])
}

/// Acceptance line 7 of slice `005`: the rung's doctor catalog entries, its
/// user documentation, and its release gates describe only behaviour this rung
/// implements. Prose honesty is enforced by review, but the part that is
/// mechanical is enforced here: the published catalog table is compared row for
/// row against the catalog the binary projects, the counts the same page states
/// are compared against it, and the three claims this rung falsified are pinned
/// so a revert cannot quietly restore them.
#[test]
fn the_published_documentation_matches_the_implemented_rung() {
    let harness = Harness::new();
    let output = harness
        .assert_command()
        .args(["doctor", "--list", "--json"])
        .output()
        .expect("list");
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    let projected: Vec<(String, String, String)> = value["checks"]
        .as_array()
        .expect("checks")
        .iter()
        .map(|row| {
            (
                row["id"].as_str().expect("id").to_owned(),
                row["scope"].as_str().expect("scope").to_owned(),
                row["severity"].as_str().expect("severity").to_owned(),
            )
        })
        .collect();

    let doctor = document("docs/reference/doctor.md");
    let documented: Vec<(String, String, String)> = section(&doctor, "\n## The catalog\n")
        .lines()
        .filter(|line| line.starts_with("| `"))
        .map(|line| {
            let cells: Vec<&str> = line.split('|').map(str::trim).collect();
            assert!(cells.len() >= 6, "unexpected catalog row: {line}");
            (
                cells[1].trim_matches('`').to_owned(),
                cells[2].to_lowercase(),
                cells[3].to_lowercase(),
            )
        })
        .collect();
    assert_eq!(
        documented, projected,
        "the published catalog table and the projected catalog must agree"
    );

    // Every count the page states about the catalog is the same number.
    let total = projected.len();
    assert!(
        doctor.contains(&format!("implemented over {total} checks")),
        "the stated catalog size must match the projected one"
    );
    assert!(doctor.contains(&format!("wrapper status=pass total={total} passed={total}")));
    assert!(doctor.contains(&format!(
        "\"summary\": {{ \"total\": {total}, \"passed\": {total}"
    )));

    // The claims this rung made false. Each was true before the account rung
    // landed, so each is a revert this gate has to catch.
    assert!(
        !doctor.contains("Slice `005` appends"),
        "the account entries are in the catalog, so the deferral sentence is stale"
    );
    assert!(
        doctor.contains("token mode still works"),
        "token mode is implemented and is not blocked by the floor, so the remediation says so"
    );
    let accounts = document("docs/reference/accounts.md");
    assert!(
        !accounts.contains("pre-implementation"),
        "the login-mode subset is implemented"
    );
    assert!(
        !accounts.contains("`project-config`"),
        "a project file cannot supply an account, so it is not a selection source"
    );
}

/// The `0.3.0` rung's own documentation gate.
///
/// Separate from the `0.2.0` one above because each rung owns its claims: this
/// one asserts the sentences the token lifecycle made false are gone, and the
/// ones it made true are present. Every assertion here is a revert this catches.
#[test]
fn the_token_lifecycle_documentation_matches_the_implemented_grammar() {
    let accounts = document("docs/reference/accounts.md");
    assert!(
        !accounts.contains("future design for slice 013"),
        "the token lifecycle shipped, so its deferral sentence is stale"
    );
    let storage = document("docs/reference/xdg-storage.md");
    assert!(
        !storage.contains("remain normative future design"),
        "the credential lock, the token file, and ordered removal all shipped"
    );
    let surface = document("docs/reference/cli-surface.md");
    assert!(
        !surface.contains("Other account subcommands and options remain normative design"),
        "every account subcommand is declared, so nothing is waiting on a later slice"
    );
    for spelling in ["`status`", "`remove`", "--token", "--stdin", "--minted-at"] {
        assert!(
            surface.contains(spelling),
            "the CLI surface must name the implemented spelling {spelling}"
        );
    }
    let output = document("docs/reference/logging-and-output.md");
    assert!(
        !output.contains("Other verb output remains normative design"),
        "only the unbuilt verbs are outstanding, and the sentence must say which"
    );
    // The one thing the wrapper still does not do on this page, kept honest so
    // a reader does not infer a helper protocol that has no specification.
    assert!(
        accounts.contains("token_helper"),
        "the unspecified helper boundary stays named rather than quietly dropped"
    );
}

/// The `0.4.0` rung's own documentation gate.
///
/// One rung, one gate: this asserts the sentences single-piece profile
/// composition and the `profile` verb made false are gone, and that `config`
/// is still honestly named as unbuilt.
#[test]
fn profile_mvp_documentation_matches_the_implemented_grammar() {
    let readme = document("README.md");
    assert!(
        !readme
            .contains("The profile isolation the crate description also promises is not shipped"),
        "profile composition shipped, so the disclaimer is stale"
    );
    assert!(
        !readme.contains("`help` and `version` are the only surfaces it owns"),
        "seven surfaces are owned, so the two-surface claim is stale"
    );

    let surface = document("docs/reference/cli-surface.md");
    assert!(
        surface.contains("`profile --help`"),
        "requested help for profile is implemented, so the surface names it"
    );
    assert!(
        !surface.contains("Other account subcommands and options remain normative design"),
        "the negative pin the token rung established stays established"
    );

    // The claims this rung made true. The 0.5.0 gate below owns what came after
    // it, so nothing here pins a wording a later rung legitimately moves.
    let configuration = document("docs/reference/configuration.md");
    assert!(
        configuration.contains("Profile resolution, piece resolution,"),
        "the configuration status paragraph must claim the shipped resolution"
    );
    assert!(
        configuration.contains("the `profile`"),
        "the configuration status paragraph must name the listing verb"
    );

    let output = document("docs/reference/logging-and-output.md");
    assert!(
        !output.contains("the unbuilt `config` and `profile` verbs"),
        "profile output shipped, so that deferral sentence is stale"
    );
    assert!(
        output.contains("the `profile`"),
        "the profile reports are named as implemented"
    );

    // The exit-code correction this rung landed, pinned at its owner.
    let codes = document("docs/reference/exit-codes.md");
    let malformed = codes
        .lines()
        .find(|line| line.contains("A resolved profile is malformed or has an empty `layers`"))
        .expect("the profile-resolution table names the malformed case");
    assert!(
        malformed.contains("`65`"),
        "the published malformed-profile row is what the implementation now exits with: {malformed}"
    );
}

/// The `0.5.0` rung's own documentation gate.
///
/// Every assertion is a revert this catches: a sentence the full composition
/// rung made false, or one it made true and that a rollback would remove.
#[test]
fn full_profile_composition_documentation_matches_the_implemented_grammar() {
    let configuration = document("docs/reference/configuration.md");
    assert!(
        !configuration.contains("remain later-slice design"),
        "every clause of the deferral sentence is now false"
    );
    assert!(
        !configuration.contains("Only the hand-maintained one exists today"),
        "all four example artifacts exist"
    );
    // Both strategy spellings and their constraints stay documented.
    for spelling in [
        "`concat`",
        "`merge-by-key`",
        "Requires exactly one non-empty `key`",
        "Never written — it is what an unlisted array does",
    ] {
        assert!(
            configuration.contains(spelling),
            "the merge table must document {spelling}"
        );
    }
    // The artifact table, by projection rather than by padded row: the column
    // widths belong to the Markdown formatter, not to this gate.
    let artifacts: Vec<(String, String, String)> = configuration
        .lines()
        .filter(|line| line.starts_with("| `"))
        .filter(|line| line.contains("example.") || line.contains("schema.json"))
        .filter(|line| line.matches('|').count() >= 5)
        .map(|line| {
            let cells: Vec<&str> = line.split('|').map(str::trim).collect();
            (
                cells[1].trim_matches('`').to_owned(),
                cells[3].to_owned(),
                cells[4].to_owned(),
            )
        })
        .collect();
    for (name, kind, present) in [
        ("config.example.toml", "Generated", "Yes"),
        ("config.schema.json", "Generated", "Yes"),
        ("profile.example.yaml", "Generated", "Yes"),
        ("piece.example.json", "Hand-maintained", "Yes"),
    ] {
        assert!(
            artifacts.iter().any(|(artifact, actual_kind, actual)| {
                artifact == name && actual_kind == kind && actual == present
            }),
            "the artifact table must list {name} as {kind}/{present}: {artifacts:?}"
        );
    }

    // The permissive unknown-key rule and its gate survive.
    assert!(
        configuration.contains("It does not reject unknown keys in the child's settings"),
        "the permissive rule is what Q-003 gates tightening of"
    );
    assert!(
        configuration.contains("`Q-003`"),
        "the gate on strict rejection stays named"
    );
    let questions = document("docs/plan/open-questions.md");
    assert!(
        questions.contains("permissive unknown-key handling remains the shaped default"),
        "Q-003 still specifies permissive handling"
    );

    // The exclusions both slices placed out of scope.
    assert!(
        configuration.contains("Where composition stops"),
        "the composed entry is not the child's whole effective configuration"
    );

    let surface = document("docs/reference/cli-surface.md");
    assert!(
        surface.contains("`config --help`"),
        "config help is a result"
    );

    let testing = document("docs/reference/testing-and-quality.md");
    assert!(
        !testing.contains("deferred — no `xtask` member yet"),
        "the gen-config gate row is backed by a hook now"
    );

    let architecture = document("docs/explanation/architecture.md");
    assert!(
        !architecture.contains("The conversion has not happened"),
        "the workspace conversion happened"
    );

    let readme = document("README.md");
    assert!(
        readme.contains("config  # what resolved, from where, and into what"),
        "the README shows the config verb"
    );
}
