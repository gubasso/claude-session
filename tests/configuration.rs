#![allow(clippy::expect_used, clippy::pedantic, clippy::nursery)]

mod support;

use std::{fs, os::unix::ffi::OsStringExt, os::unix::fs::PermissionsExt};
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
        .bound_command()
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
    let mut command = harness.bound_command();
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
            .bound_command()
            .args(["--config".into(), path.into_os_string()])
            .status()
            .expect("wrapper")
            .success()
    );
}

/// The project layer had only negative coverage, so nothing proved a file
/// inside a repository is ever found. A discovery that silently matched
/// nothing would have passed every test that existed.
///
/// The refusal of a forbidden key is the detector: only a file that was found
/// and read can be rejected for what it contains.
#[test]
fn a_project_file_inside_a_repository_is_read() {
    let harness = Harness::new();
    let repo = harness.root().join("repo");
    let nested = repo.join("nested");
    fs::create_dir_all(repo.join(".git")).expect("marker");
    fs::create_dir_all(&nested).expect("cwd");
    fs::write(repo.join(".claude-session.toml"), "child_bin='/anything'\n").expect("project file");
    let mut command = harness.command();
    command.current_dir(&nested);
    let output = command.output().expect("wrapper");
    assert_eq!(output.status.code(), Some(78));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not allowed in a project file"),
        "the project file was never read:\n{stderr}"
    );
}

#[test]
fn a_project_file_cannot_select_an_account() {
    let harness = Harness::new();
    let repo = harness.root().join("repo");
    fs::create_dir_all(repo.join(".git")).expect("marker");
    fs::write(
        repo.join(".claude-session.toml"),
        "default_account='work'\n",
    )
    .expect("project file");
    let mut command = harness.command();
    command.current_dir(&repo);
    let output = command.output().expect("wrapper");
    assert_eq!(output.status.code(), Some(78));
    assert!(String::from_utf8_lossy(&output.stderr).contains("default_account"));
}

/// Outside a repository there is no project layer at all. Without a ceiling the
/// walk reaches the filesystem root, so a file in the home directory applies to
/// every invocation made from an unrelated tree.
#[test]
fn no_project_layer_applies_outside_a_repository() {
    let harness = Harness::new();
    let nested = harness.root().join("loose/nested");
    fs::create_dir_all(&nested).expect("cwd");
    fs::write(
        harness.root().join("loose/.claude-session.toml"),
        "child_bin='/not/allowed'\n",
    )
    .expect("stray file");
    let mut command = harness.bound_command();
    command.current_dir(&nested);
    assert!(
        command.status().expect("wrapper").success(),
        "a project file outside any repository was read"
    );
}

/// A file the user explicitly named has three distinct failures with three
/// distinct fixes. Collapsing them into one code tells the reader to correct a
/// value when the real problem is that the file cannot be opened at all.
#[test]
fn an_unreadable_named_config_is_typed() {
    let harness = Harness::new();
    let path = harness.root().join("unreadable.toml");
    fs::write(&path, "default_profile='x'\n").expect("config");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).expect("mode");
    let output = harness
        .command()
        .args(["--config".into(), path.clone().into_os_string()])
        .output()
        .expect("wrapper");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(77),
        "an unreadable file reported {:?}:\n{stderr}",
        output.status.code()
    );
    assert!(stderr.contains("Permission"), "{stderr}");
    assert!(
        stderr.contains(path.to_str().expect("utf8")),
        "the diagnostic did not name the file:\n{stderr}"
    );
}

/// An identifier becomes a path component in the storage tree, so its grammar
/// is a usage contract every layer shares. Before this, a bad name in a file
/// exited `Config` and the same name on the command line exited `Usage` — two
/// codes for one mistake, disagreeing with the two pages that own the rule.
#[test]
fn an_invalid_identifier_from_any_layer_exits_usage() {
    let harness = Harness::new();
    let config = harness.root().join("bad-profile.toml");
    fs::write(&config, "default_profile='Work'\n").expect("config");
    let output = harness
        .command()
        .args(["--config".into(), config.clone().into_os_string()])
        .output()
        .expect("wrapper");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(64), "user file layer:\n{stderr}");
    assert!(stderr.contains("Work"), "{stderr}");
    assert!(
        stderr.contains(config.to_str().expect("utf8")),
        "the diagnostic did not name the file:\n{stderr}"
    );

    let output = harness
        .command()
        .env("CLAUDE_SESSION_DEFAULT_ACCOUNT", "has.dot")
        .output()
        .expect("wrapper");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(64),
        "environment layer:\n{stderr}"
    );
    assert!(stderr.contains("has.dot"), "{stderr}");
    assert!(
        stderr.contains("CLAUDE_SESSION_DEFAULT_ACCOUNT"),
        "the diagnostic did not name the variable:\n{stderr}"
    );

    let output = harness
        .command()
        .args(["--profile", "has/slash"])
        .output()
        .expect("wrapper");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(64), "command line:\n{stderr}");
    assert!(stderr.contains("has/slash"), "{stderr}");
}

// --- The config report verb -------------------------------------------------

const CONFIG_PIECE: &str = r#"{"model":"sonnet","permissions":{"allow":["x"]}}"#;
const CONFIG_PROFILE: &str = "layers:\n  - base\n";

/// Runs `config --json` and returns the parsed document with the exit code.
fn config_json(command: &mut std::process::Command) -> (serde_json::Value, Option<i32>) {
    let output = command.output().expect("config runs");
    let document = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "config --json did not emit one document ({error}): {}",
            String::from_utf8_lossy(&output.stdout)
        )
    });
    (document, output.status.code())
}

/// Three keys, three different winning layers, so the provenance reported is
/// per key rather than one answer for the whole file.
#[test]
fn config_reports_every_key_with_its_winning_layer() {
    let harness = Harness::new();
    let file = harness.root().join("config.toml");
    fs::write(&file, "default_account = \"work\"\n").expect("config");
    harness.write_piece("base", CONFIG_PIECE);
    harness.write_profile("dev", CONFIG_PROFILE);

    let (document, code) = config_json(
        harness
            .command()
            .args(["--config".into(), file.clone().into_os_string()])
            .args(["--profile", "dev", "config", "--json"]),
    );
    assert_eq!(code, Some(0));
    let configuration = &document["configuration"];
    // The harness points the wrapper at its recording stub through the
    // environment, which is exactly the environment layer.
    assert_eq!(configuration["child_bin"]["source"], "environment");
    assert_eq!(configuration["default_account"]["source"], "user-config");
    assert_eq!(configuration["default_account"]["value"], "work");
    assert_eq!(configuration["default_profile"]["source"], "cli");
    assert_eq!(configuration["default_profile"]["value"], "dev");
    assert!(
        document.get("schema_version").is_none(),
        "schema_version appears on doctor alone"
    );
}

/// An unset key carries its `default` provenance and no value at all.
#[test]
fn config_reports_an_unset_key_without_a_value() {
    let harness = Harness::new();
    let (document, code) = config_json(harness.command().args(["config", "--json"]));
    assert_eq!(code, Some(0));
    let account = &document["configuration"]["default_account"];
    assert_eq!(account["source"], "default");
    assert!(
        account.get("value").is_none(),
        "an absent optional field is omitted, never null"
    );
    assert!(
        document.get("profile").is_none(),
        "no profile resolved, so the section is omitted entirely"
    );
}

#[test]
fn config_names_the_files_it_consulted_and_whether_they_existed() {
    let harness = Harness::new();
    let file = harness.root().join("absent.toml");
    let (document, code) = config_json(
        harness
            .command()
            .args(["--config".into(), file.clone().into_os_string()])
            .args(["config", "--json"]),
    );
    assert_eq!(code, Some(0));
    let files = document["files"].as_array().expect("files array");
    let row = files
        .iter()
        .find(|row| row["layer"] == "user-config")
        .expect("the user layer is always consulted");
    assert_eq!(row["path"], file.display().to_string());
    assert_eq!(row["existed"], false);
}

#[test]
fn config_reports_the_active_profile_its_pieces_and_the_entry_path() {
    let harness = Harness::new();
    harness.write_piece("base", CONFIG_PIECE);
    harness.write_profile("dev", CONFIG_PROFILE);
    let (document, code) =
        config_json(
            harness
                .command()
                .args(["--profile", "dev", "config", "--json"]),
        );
    assert_eq!(code, Some(0));
    let profile = &document["profile"];
    assert_eq!(profile["name"], "dev");
    assert_eq!(profile["pieces"][0]["name"], "base");
    let settings = profile["entry"]["settings"]
        .as_str()
        .expect("settings path");
    let name = settings.rsplit('/').next().expect("file name");
    assert!(
        name.starts_with("profile-dev-") && name.ends_with(".json"),
        "{name} is not the composed entry grammar"
    );
    let stem = name
        .trim_start_matches("profile-dev-")
        .trim_end_matches(".json");
    assert_eq!(stem.len(), 12, "{name}");
    assert!(stem.bytes().all(|byte| byte.is_ascii_hexdigit()), "{name}");
}

/// The read-only proof: describing an entry must not bring it into being.
#[test]
fn config_exits_zero_when_the_entry_is_not_yet_written() {
    let harness = Harness::new();
    harness.write_piece("base", CONFIG_PIECE);
    harness.write_profile("dev", CONFIG_PROFILE);
    let (document, code) =
        config_json(
            harness
                .command()
                .args(["--profile", "dev", "config", "--json"]),
        );
    assert_eq!(code, Some(0));
    assert_eq!(document["profile"]["entry"]["exists"], false);
    assert!(
        !harness.state().join("composed").exists(),
        "the report created the composed store"
    );
}

#[test]
fn config_exits_with_the_defects_code_when_a_profile_is_malformed() {
    let harness = Harness::new();
    harness.write_piece("base", CONFIG_PIECE);
    harness.write_profile("dev", "layers: [\n");
    let output = harness
        .command()
        .args(["--profile", "dev", "config"])
        .output()
        .expect("config runs");
    assert_eq!(output.status.code(), Some(65));
    assert!(
        !output.stdout.is_empty(),
        "the report is rendered before the exit is decided"
    );
}

/// `config` is an assertion verb, so unresolvable wrapper configuration is the
/// answer it exists to render rather than a reason to abandon the report. It
/// previously failed at the boundary, leaving its `wrapper-config-parses`
/// defect row unreachable.
#[test]
fn config_reports_a_malformed_wrapper_configuration_as_a_defect() {
    let harness = Harness::new();
    let config = harness.root().join("broken.toml");
    fs::write(&config, "child_bin = \n").expect("config");
    let output = harness
        .command()
        .args(["--config".into(), config.into_os_string()])
        .arg("config")
        .arg("--json")
        .output()
        .expect("config runs");
    assert!(
        !output.stdout.is_empty(),
        "the report is rendered before the exit is decided"
    );
    let document: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("the report is still JSON");
    let defect = document["defects"]
        .as_array()
        .expect("defects is an array")
        .iter()
        .find(|entry| entry["id"] == "wrapper-config-parses")
        .expect("the catalog row is reported");
    assert_eq!(defect["status"], "fail");
    assert_eq!(output.status.code(), Some(78));
}

#[test]
fn config_exits_with_no_input_when_a_piece_is_missing() {
    let harness = Harness::new();
    harness.write_profile("dev", CONFIG_PROFILE);
    let output = harness
        .command()
        .args(["--profile", "dev", "config"])
        .output()
        .expect("config runs");
    assert_eq!(output.status.code(), Some(66));
    assert!(!output.stdout.is_empty());
}

/// The ADR-0018 parity requirement: one catalog, one remediation, so the two
/// verbs cannot instruct a user differently about one defect.
#[test]
fn config_quotes_the_catalog_remediation_verbatim() {
    let harness = Harness::new();
    harness.write_piece("base", CONFIG_PIECE);
    harness.write_profile("dev", "layers: []\n");
    let (config, _) = config_json(
        harness
            .command()
            .args(["--profile", "dev", "config", "--json"]),
    );
    let (doctor, _) = config_json(
        harness
            .command()
            .env("XDG_RUNTIME_DIR", harness.root().join("runtime"))
            .args(["--profile", "dev", "doctor", "--json"]),
    );
    let hint = |rows: &serde_json::Value| {
        rows.as_array()
            .expect("rows")
            .iter()
            .find(|row| row["id"] == "settings-profile-valid")
            .and_then(|row| row["hint"].as_str())
            .map(str::to_owned)
    };
    let from_config = hint(&config["defects"]).expect("config quotes the defect");
    let from_doctor = hint(&doctor["wrapper"]["checks"]).expect("doctor quotes the defect");
    assert_eq!(from_config, from_doctor);
}

#[test]
fn config_json_and_human_reports_carry_the_same_facts() {
    let harness = Harness::new();
    harness.write_piece("base", CONFIG_PIECE);
    harness.write_profile("dev", CONFIG_PROFILE);
    let human = harness
        .command()
        .args(["--profile", "dev", "config"])
        .output()
        .expect("config runs");
    let text = String::from_utf8(human.stdout).expect("utf-8 report");
    let (document, _) =
        config_json(
            harness
                .command()
                .args(["--profile", "dev", "config", "--json"]),
        );
    let flowed = support::flowed(&text);
    // The same facts, said rather than labelled: the resolved value, the layer
    // that supplied it, and the pieces the profile composes (ADR-0093).
    assert!(
        flowed.contains("default_profile is dev, from the command line."),
        "{text}"
    );
    assert!(
        flowed.contains("It composes one settings piece, in the order it lists them: base."),
        "{text}"
    );
    for field in ["default_profile:", "piece:", "digest:", "[cli]"] {
        assert!(!text.contains(field), "{field} survives in {text}");
    }
    // The full digest is machine data: a person cannot act on it, and the
    // machine document carries it. The path it keys, which a person can open,
    // is what the human form names instead.
    assert!(
        !text.contains(
            document["profile"]["entry"]["digest"]
                .as_str()
                .expect("digest")
        ),
        "{text}"
    );
    assert!(
        text.contains(
            document["profile"]["entry"]["settings"]
                .as_str()
                .expect("settings")
        ),
        "{text}"
    );
    // A check id belongs to the machine document, which carries every one of
    // them; the human form carries the reason and the next action instead.
    for defect in document["defects"].as_array().expect("defects") {
        let id = defect["id"].as_str().expect("id");
        assert!(!text.contains(id), "the human form names {id}: {text}");
        if let Some(reason) = defect["reason"].as_str() {
            assert!(
                flowed.contains(&support::flowed(reason)),
                "the human form omits why {id} applies: {text}"
            );
        }
    }
    // Never claims the composed entry is the child's whole configuration.
    assert!(
        flowed.contains("not claude's whole effective configuration"),
        "{text}"
    );
}

#[test]
fn config_writes_data_to_stdout_and_diagnostics_to_stderr() {
    let harness = Harness::new();
    harness.write_piece("base", CONFIG_PIECE);
    harness.write_profile("dev", CONFIG_PROFILE);
    let output = harness
        .command()
        .args(["--profile", "dev", "config"])
        .output()
        .expect("config runs");
    assert!(!output.stdout.is_empty());
    assert!(
        output.stderr.is_empty(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// The full profile report: one profile's ordered layers, resolved paths, and
/// the strategies it declares.
#[test]
fn config_reports_one_profiles_ordered_layers_and_resolved_paths() {
    let harness = Harness::new();
    harness.write_piece("base", CONFIG_PIECE);
    harness.write_piece("extra", r#"{"permissions":{"allow":["y"]}}"#);
    harness.write_profile(
        "work",
        concat!(
            "layers:\n  - base\n  - extra\n",
            "array_strategies:\n  \"/permissions/allow\":\n    strategy: concat\n"
        ),
    );
    let (document, code) =
        config_json(
            harness
                .command()
                .args(["--profile", "work", "config", "--json"]),
        );
    assert_eq!(code, Some(0));
    let profile = &document["profile"];
    let pieces = profile["pieces"].as_array().expect("pieces");
    assert_eq!(pieces.len(), 2);
    assert_eq!(pieces[0]["name"], "base");
    assert_eq!(pieces[1]["name"], "extra");
    assert!(
        pieces[0]["path"]
            .as_str()
            .expect("path")
            .ends_with("/settings/base.json")
    );
    let strategies = profile["strategies"].as_array().expect("strategies");
    assert_eq!(strategies.len(), 1);
    assert_eq!(strategies[0]["pointer"], "/permissions/allow");
    assert_eq!(strategies[0]["strategy"], "concat");
    assert!(
        strategies[0].get("key").is_none(),
        "concat takes no key, so the field is omitted"
    );
}

#[test]
fn config_help_is_a_result_on_standard_output() {
    let harness = Harness::new();
    let flag = harness
        .command()
        .args(["config", "--help"])
        .output()
        .expect("config --help runs");
    let verb = harness
        .command()
        .args(["help", "config"])
        .output()
        .expect("help config runs");
    assert_eq!(flag.status.code(), Some(0));
    assert_eq!(verb.status.code(), Some(0));
    assert!(!flag.stdout.is_empty());
    assert_eq!(flag.stdout, verb.stdout);
}

#[test]
fn config_never_reaches_the_child() {
    let harness = Harness::new();
    harness
        .command()
        .arg("config")
        .output()
        .expect("config runs");
    assert!(
        !harness.record_dir().join("argv").exists(),
        "the config verb is answered by the wrapper"
    );
}
