//! Secure session storage, observed through the binary.
//!
//! Every behaviour here touches a filesystem, so it belongs in the integration
//! lane by `docs/reference/testing-and-quality.md`. The reachable surface is the
//! bare passthrough launch: it prepares the account directories and materialises
//! the composed entry, and reports both as `debug` records.

#![allow(clippy::expect_used, clippy::pedantic, clippy::nursery)]

mod support;

use std::{fs, os::unix::fs::PermissionsExt, path::Path, time::SystemTime};

use support::{Harness, entries, mode, read_nul};

const PIECE_BASE: &str = r#"{"model":"sonnet","env":{"A":"1"}}"#;
const PIECE_WORK: &str = r#"{"env":{"B":"2"},"allow":["x"]}"#;
const PROFILE_WORK: &str = "layers:\n  - base\n  - work\n";

/// Writes the two pieces and the profile every composition test starts from.
fn fixtures(harness: &Harness) {
    harness.write_piece("base", PIECE_BASE);
    harness.write_piece("work", PIECE_WORK);
    harness.write_profile("work", PROFILE_WORK);
}

fn composed(harness: &Harness) -> std::path::PathBuf {
    harness.state().join("composed")
}

fn modified(path: &Path) -> SystemTime {
    fs::symlink_metadata(path)
        .expect("fixture metadata")
        .modified()
        .expect("mtime")
}

/// The entry pair in the store, as `(settings, provenance)`.
fn pair(harness: &Harness) -> (std::path::PathBuf, std::path::PathBuf) {
    let store = composed(harness);
    let names = entries(&store);
    let settings: Vec<&String> = names
        .iter()
        .filter(|name| name.ends_with(".json") && !name.ends_with(".compose.json"))
        .collect();
    assert_eq!(settings.len(), 1, "expected one settings file in {names:?}");
    let stem = settings[0].trim_end_matches(".json").to_owned();
    (
        store.join(format!("{stem}.json")),
        store.join(format!("{stem}.compose.json")),
    )
}

/// The whole acceptance sentence is "a symlink or the wrong owner"; the
/// ownership leg cannot be built without root, so it is proven by
/// `services::storage::guard::tests::a_foreign_owner_is_refused` over the same
/// predicate this path reaches.
#[test]
fn a_symlinked_managed_component_is_refused_before_its_leaf() {
    let harness = Harness::new();
    let elsewhere = harness.root().join("elsewhere");
    fs::create_dir_all(&elsewhere).expect("link target");
    fs::create_dir_all(harness.state()).expect("namespace");
    let linked = harness.state().join("accounts");
    std::os::unix::fs::symlink(&elsewhere, &linked).expect("symlink");

    let output = harness
        .command()
        .args(["--account", "work"])
        .output()
        .expect("wrapper");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(77), "{stderr}");
    assert!(stderr.contains("storage-paths-no-symlinks"), "{stderr}");
    assert!(
        stderr.contains(linked.to_str().expect("utf8")),
        "the diagnostic did not name the link:\n{stderr}"
    );
    assert!(
        stderr.contains("Move the symbolic link at"),
        "the hint was not the catalog's wording:\n{stderr}"
    );
    // Refused before its leaf: nothing was created through the link.
    assert!(
        !elsewhere.join("work").exists(),
        "the account directory was created through the link"
    );
}

/// Every invocation, not only creation. An implementation that sets the mode
/// once at `mkdir` passes the first leg and fails the second.
#[test]
fn an_over_permissive_managed_directory_is_corrected_on_every_invocation() {
    let harness = Harness::new();
    harness.initialize_login("work");
    assert!(
        harness
            .command()
            .args(["--account", "work"])
            .status()
            .expect("wrapper")
            .success()
    );
    let account = harness.state().join("accounts/work");
    assert_eq!(mode(&account), 0o700);

    for drifted in [0o777, 0o755] {
        fs::set_permissions(&account, fs::Permissions::from_mode(drifted)).expect("drift");
        let output = harness
            .command()
            .args(["--account", "work", "--verbose", "--verbose"])
            .output()
            .expect("wrapper");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "a correction is a pass, not a failure:\n{stderr}"
        );
        assert_eq!(mode(&account), 0o700, "mode after {drifted:o}");
        assert!(stderr.contains("storage-directory-modes"), "{stderr}");
        assert!(
            stderr.contains("status=pass"),
            "a repair must report pass:\n{stderr}"
        );
    }
}

/// Reuse means composing nothing, not rewriting identical bytes — which is why
/// the modification time is asserted alongside the content.
#[test]
fn identical_inputs_name_and_reuse_one_immutable_entry() {
    let harness = Harness::new();
    fixtures(&harness);
    assert!(
        harness
            .command()
            .args(["--profile", "work"])
            .status()
            .expect("wrapper")
            .success()
    );
    let (settings, provenance) = pair(&harness);
    let before = (
        fs::read(&settings).expect("settings"),
        fs::read(&provenance).expect("provenance"),
        modified(&settings),
        modified(&provenance),
    );
    assert_eq!(mode(&settings), 0o600);
    assert_eq!(mode(&provenance), 0o600);

    assert!(
        harness
            .command()
            .args(["--profile", "work"])
            .status()
            .expect("wrapper")
            .success()
    );
    assert_eq!(
        entries(&composed(&harness)).len(),
        2,
        "identical inputs named a second entry"
    );
    assert_eq!(fs::read(&settings).expect("settings"), before.0);
    assert_eq!(fs::read(&provenance).expect("provenance"), before.1);
    assert_eq!(modified(&settings), before.2, "the settings were rewritten");
    assert_eq!(
        modified(&provenance),
        before.3,
        "the provenance was rewritten"
    );

    let sidecar: serde_json::Value =
        serde_json::from_slice(&before.1).expect("the sidecar is JSON");
    let digest = sidecar["digest"].as_str().expect("digest");
    assert_eq!(digest.len(), 64);
    assert!(
        digest
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    );
    let name = settings.file_name().expect("name").to_string_lossy();
    assert!(
        name.ends_with(&format!("{}.json", &digest[..12])),
        "{name} is not named by the first twelve characters of {digest}"
    );
    assert!(name.starts_with("profile-work-"), "{name}");
}

/// The refusal is what makes "two profiles never share settings" a check
/// rather than a probability, so it must not open or overwrite the entry.
#[test]
fn a_sidecar_digest_mismatch_is_refused_without_overwrite() {
    let harness = Harness::new();
    fixtures(&harness);
    assert!(
        harness
            .command()
            .args(["--profile", "work"])
            .status()
            .expect("wrapper")
            .success()
    );
    let (settings, provenance) = pair(&harness);
    let mut sidecar: serde_json::Value =
        serde_json::from_slice(&fs::read(&provenance).expect("provenance")).expect("json");
    sidecar["digest"] = serde_json::Value::String("0".repeat(64));
    fs::write(&provenance, serde_json::to_vec(&sidecar).expect("json")).expect("rewrite");
    let before = (fs::read(&settings).expect("settings"), modified(&settings));

    let output = harness
        .command()
        .args(["--profile", "work"])
        .output()
        .expect("wrapper");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(65), "{stderr}");
    assert!(stderr.contains("settings-entry-consistent"), "{stderr}");
    assert!(
        stderr.contains(settings.to_str().expect("utf8")),
        "the diagnostic did not name the entry:\n{stderr}"
    );
    assert_eq!(fs::read(&settings).expect("settings"), before.0);
    assert_eq!(modified(&settings), before.1, "the settings were touched");
    assert!(
        entries(&composed(&harness))
            .iter()
            .all(|name| !name.ends_with(".tmp")),
        "a temporary survived a refused run"
    );
}

/// A settings file without its provenance carries no digest to compare, so
/// adopting the survivor would turn the guarantee above back into a
/// probability. The wrong implementation this rejects is keeping it.
#[test]
fn a_half_written_pair_is_replaced_from_this_runs_inputs() {
    let harness = Harness::new();
    fixtures(&harness);
    assert!(
        harness
            .command()
            .args(["--profile", "work"])
            .status()
            .expect("wrapper")
            .success()
    );
    let (settings, provenance) = pair(&harness);
    fs::remove_file(&provenance).expect("remove the sidecar");
    fs::write(&settings, r#"{"marker":"survivor"}"#).expect("survivor");

    assert!(
        harness
            .command()
            .args(["--profile", "work"])
            .status()
            .expect("wrapper")
            .success()
    );
    assert!(provenance.exists(), "the sidecar was not rewritten");
    let body = fs::read_to_string(&settings).expect("settings");
    assert!(
        !body.contains("survivor"),
        "the survivor was adopted: {body}"
    );
    assert!(body.contains("sonnet"), "{body}");
}

/// Replacing a survivor is writing its leaf, so the acceptance sentence binds
/// it: a linked survivor is refused before the rename, not renamed over.
#[test]
fn a_symlinked_partial_pair_survivor_is_refused_before_it_is_replaced() {
    let harness = Harness::new();
    fixtures(&harness);
    assert!(
        harness
            .command()
            .args(["--profile", "work"])
            .status()
            .expect("wrapper")
            .success()
    );
    let (settings, provenance) = pair(&harness);
    fs::remove_file(&provenance).expect("remove the sidecar");
    fs::remove_file(&settings).expect("remove the survivor");
    let elsewhere = harness.root().join("elsewhere.json");
    fs::write(&elsewhere, r#"{"marker":"outside"}"#).expect("link target");
    std::os::unix::fs::symlink(&elsewhere, &settings).expect("symlink");

    let output = harness
        .command()
        .args(["--profile", "work"])
        .output()
        .expect("wrapper");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(77), "{stderr}");
    assert!(stderr.contains("storage-paths-no-symlinks"), "{stderr}");
    assert_eq!(
        fs::read_to_string(&elsewhere).expect("target"),
        r#"{"marker":"outside"}"#,
        "the run wrote through the link"
    );
    assert!(
        fs::symlink_metadata(&settings)
            .expect("survivor")
            .file_type()
            .is_symlink(),
        "the link was replaced instead of refused"
    );
}

/// Drift on a composed file is the `storage-secret-modes` row, and that row
/// reads "correct, then proceed" exactly as the directory row does.
#[test]
fn an_over_permissive_composed_file_is_corrected_and_the_run_proceeds() {
    let harness = Harness::new();
    fixtures(&harness);
    assert!(
        harness
            .command()
            .args(["--profile", "work"])
            .status()
            .expect("wrapper")
            .success()
    );
    let (settings, provenance) = pair(&harness);
    let before = fs::read(&settings).expect("settings");
    for path in [&settings, &provenance] {
        fs::set_permissions(path, fs::Permissions::from_mode(0o644)).expect("drift");
    }

    let output = harness
        .command()
        .args(["--profile", "work"])
        .output()
        .expect("wrapper");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "a correction is a pass, not a failure:\n{stderr}"
    );
    assert_eq!(mode(&settings), 0o600);
    assert_eq!(mode(&provenance), 0o600);
    assert_eq!(
        fs::read(&settings).expect("settings"),
        before,
        "a corrected entry was rewritten"
    );
}

/// The deciding case for keying by inputs: two profiles launched from one
/// terminal must never meet.
#[test]
fn two_profiles_in_one_terminal_name_different_entries() {
    let harness = Harness::new();
    harness.write_piece("base", PIECE_BASE);
    harness.write_piece("work", PIECE_WORK);
    harness.write_profile("work", PROFILE_WORK);
    harness.write_profile("home", "layers:\n  - base\n");

    for profile in ["work", "home"] {
        assert!(
            harness
                .command()
                .args(["--profile", profile])
                .status()
                .expect("wrapper")
                .success()
        );
    }
    let names = entries(&composed(&harness));
    assert_eq!(names.len(), 4, "expected two complete pairs: {names:?}");
    assert!(names.iter().any(|name| name.starts_with("profile-work-")));
    assert!(names.iter().any(|name| name.starts_with("profile-home-")));
}

/// No input is account-scoped, so the account cannot enter the key.
#[test]
fn identical_inputs_under_different_accounts_name_one_entry() {
    let harness = Harness::new();
    fixtures(&harness);
    for account in ["a", "b"] {
        harness.initialize_login(account);
        assert!(
            harness
                .command()
                .args(["--account", account, "--profile", "work"])
                .status()
                .expect("wrapper")
                .success()
        );
    }
    assert_eq!(
        entries(&composed(&harness)).len(),
        2,
        "the account reached the entry key"
    );
}

/// Existence is the whole freshness answer: editing a piece names a different
/// entry, and the old one is immutable and stays.
#[test]
fn editing_a_piece_names_a_new_entry_and_leaves_the_old_one() {
    let harness = Harness::new();
    fixtures(&harness);
    assert!(
        harness
            .command()
            .args(["--profile", "work"])
            .status()
            .expect("wrapper")
            .success()
    );
    let (first, _) = pair(&harness);
    harness.write_piece("work", r#"{"env":{"B":"changed"}}"#);
    assert!(
        harness
            .command()
            .args(["--profile", "work"])
            .status()
            .expect("wrapper")
            .success()
    );
    assert!(first.exists(), "the old entry was removed");
    assert_eq!(
        entries(&composed(&harness)).len(),
        4,
        "an edited piece did not name a new entry"
    );
}

/// A wrapper-managed component begins at the namespace directory. Walking above
/// it would apply the wrapper's policy to the user's own tree.
#[test]
fn unmanaged_ancestors_are_never_checked_or_corrected() {
    let harness = Harness::new();
    fixtures(&harness);
    harness.initialize_login("work");
    let state_base = harness.root().join("state");
    let home = harness.root().join("home");
    for path in [&state_base, &home] {
        fs::create_dir_all(path).expect("ancestor");
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).expect("ancestor mode");
    }
    assert!(
        harness
            .command()
            .args(["--account", "work", "--profile", "work"])
            .status()
            .expect("wrapper")
            .success()
    );
    assert_eq!(mode(&state_base), 0o755, "the XDG state base was corrected");
    assert_eq!(mode(&home), 0o755, "the home directory was corrected");
    assert_eq!(mode(&harness.state()), 0o700);
}

#[test]
fn a_wrong_typed_managed_path_is_refused() {
    let harness = Harness::new();
    fixtures(&harness);
    fs::create_dir_all(harness.state()).expect("namespace");
    fs::write(harness.state().join("composed"), b"not a directory").expect("intruder");

    let output = harness
        .command()
        .args(["--profile", "work"])
        .output()
        .expect("wrapper");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(77), "{stderr}");
    assert!(stderr.contains("storage-paths-typed"), "{stderr}");
    assert!(stderr.contains("is a regular file"), "{stderr}");
    assert!(stderr.contains("must be a directory"), "{stderr}");
}

/// A live process id means a concurrent writer, whose rename the sweep must
/// never break. A lock file is not a temporary at all.
#[test]
fn an_orphaned_temporary_is_swept_and_a_live_one_is_kept() {
    let harness = Harness::new();
    fixtures(&harness);
    fs::create_dir_all(composed(&harness)).expect("store");
    let store = composed(&harness);
    let orphan = store.join(".profile-work-abc.json.4294967290.tmp");
    let live = store.join(format!(".profile-work-abc.json.{}.tmp", std::process::id()));
    let lock = store.join(".credentials.lock");
    for path in [&orphan, &live, &lock] {
        fs::write(path, b"x").expect("fixture");
    }

    assert!(
        harness
            .command()
            .args(["--profile", "work"])
            .status()
            .expect("wrapper")
            .success()
    );
    assert!(!orphan.exists(), "the orphan survived the sweep");
    assert!(live.exists(), "a live writer's temporary was removed");
    assert!(lock.exists(), "a lock file was swept");
}

/// The rename either happened or it did not, so the final path always holds a
/// complete file and a leftover temporary is inert.
#[test]
fn an_interrupted_write_leaves_the_previous_complete_file() {
    let harness = Harness::new();
    fixtures(&harness);
    assert!(
        harness
            .command()
            .args(["--profile", "work"])
            .status()
            .expect("wrapper")
            .success()
    );
    let (settings, _) = pair(&harness);
    let name = settings.file_name().expect("name").to_string_lossy();
    fs::write(
        composed(&harness).join(format!(".{name}.4294967290.tmp")),
        b"half written junk",
    )
    .expect("temporary");

    assert!(
        harness
            .command()
            .args(["--profile", "work"])
            .status()
            .expect("wrapper")
            .success()
    );
    let body = fs::read_to_string(&settings).expect("settings");
    let parsed: serde_json::Value = serde_json::from_str(&body).expect("the entry still parses");
    assert_eq!(parsed["model"], "sonnet");
}

/// Laziness and the passthrough contract, in one test: with nothing selected
/// the wrapper touches no storage and the child sees the argument vector it
/// would have seen without the wrapper.
#[test]
fn an_empty_config_tree_launches_the_child_unchanged() {
    let harness = Harness::new();
    assert!(
        harness
            .command()
            .args(["--help-child", "extra"])
            .status()
            .expect("wrapper")
            .success()
    );
    assert_eq!(
        entries(&harness.state()),
        vec!["claude-session.log".to_owned()],
        "an unselected run wrote to the state namespace"
    );
    let argv = read_nul(&harness.record_dir().join("argv"));
    let rendered: Vec<String> = argv
        .iter()
        .map(|value| String::from_utf8_lossy(value).into_owned())
        .collect();
    assert_eq!(&rendered[1..], ["--help-child", "extra"]);
}

/// The entry the run resolved is the one the child is told to read, and it
/// arrives as a two-token prefix ahead of an untouched suffix.
#[test]
fn a_selected_profile_reaches_the_child_as_a_settings_prefix() {
    let harness = Harness::new();
    fixtures(&harness);
    assert!(
        harness
            .command()
            .args(["--profile", "work", "run"])
            .status()
            .expect("wrapper")
            .success()
    );
    let argv = read_nul(&harness.record_dir().join("argv"));
    let rendered: Vec<String> = argv
        .iter()
        .map(|value| String::from_utf8_lossy(value).into_owned())
        .collect();
    let entries = entries(&composed(&harness));
    let settings = entries
        .iter()
        .find(|name| name.starts_with("profile-work-") && !name.ends_with(".compose.json"))
        .expect("the composed settings document");
    assert_eq!(
        &rendered[1..],
        [
            "--settings".to_owned(),
            composed(&harness).join(settings).display().to_string(),
            "run".to_owned()
        ]
    );
}

/// A profile the user asked for and did not get is the silent-wrong-settings
/// bug, so a missing piece names both the profile and the path it looked for.
#[test]
fn a_missing_piece_names_the_profile_and_the_path() {
    let harness = Harness::new();
    harness.write_piece("base", PIECE_BASE);
    harness.write_profile("work", PROFILE_WORK);

    let output = harness
        .command()
        .args(["--profile", "work"])
        .output()
        .expect("wrapper");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(66), "{stderr}");
    assert!(stderr.contains("settings-compose"), "{stderr}");
    assert!(stderr.contains("work.json"), "{stderr}");
    assert!(stderr.contains("profile work"), "{stderr}");
}

/// The published `exit-codes.md` row: a profile that exists but does not parse
/// is `DataFormat`, not `NoInput`. It is a distinct failure from a missing one,
/// and the check that reports it says so.
#[test]
fn a_malformed_profile_is_a_data_format_error() {
    let harness = Harness::new();
    harness.write_piece("base", PIECE_BASE);
    harness.write_profile("work", "layers: [\n");

    let output = harness
        .command()
        .args(["--profile", "work"])
        .output()
        .expect("wrapper");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(65), "{stderr}");
    assert!(stderr.contains("settings-profile-valid"), "{stderr}");
    assert!(stderr.contains("work.yaml"), "{stderr}");
}

#[test]
fn an_empty_layer_list_is_a_data_format_error() {
    let harness = Harness::new();
    harness.write_piece("base", PIECE_BASE);
    harness.write_profile("work", "layers: []\n");

    let output = harness
        .command()
        .args(["--profile", "work"])
        .output()
        .expect("wrapper");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(65), "{stderr}");
    assert!(stderr.contains("settings-profile-valid"), "{stderr}");
    assert!(stderr.contains("work.yaml"), "{stderr}");
}

/// The boundary the two tests above move: existence stays `settings-compose`
/// and `NoInput`, so the split is proven rather than assumed.
#[test]
fn a_missing_profile_is_still_a_no_input_error() {
    let harness = Harness::new();
    harness.write_piece("base", PIECE_BASE);

    let output = harness
        .command()
        .args(["--profile", "work"])
        .output()
        .expect("wrapper");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(66), "{stderr}");
    assert!(stderr.contains("settings-compose"), "{stderr}");
}

/// The one-piece acceptance sentence, whole: deterministic bytes, the private
/// mode, and a sidecar naming the profile and its single piece. `keys` is
/// asserted absent, because an empty contributor map would claim an answer this
/// rung's fold cannot give.
#[test]
fn one_piece_profile_writes_exact_settings_and_basic_sidecar() {
    let harness = Harness::new();
    harness.write_piece("base", PIECE_BASE);
    harness.write_profile("solo", "layers:\n  - base\n");

    assert!(
        harness
            .command()
            .args(["--profile", "solo"])
            .status()
            .expect("wrapper")
            .success()
    );
    let (settings, provenance) = pair(&harness);
    assert_eq!(mode(&settings), 0o600);
    assert_eq!(mode(&provenance), 0o600);

    let composed_document: serde_json::Value =
        serde_json::from_slice(&fs::read(&settings).expect("settings")).expect("settings is JSON");
    let piece: serde_json::Value = serde_json::from_str(PIECE_BASE).expect("piece is JSON");
    assert_eq!(
        composed_document, piece,
        "one piece composes to exactly itself"
    );

    let sidecar: serde_json::Value =
        serde_json::from_slice(&fs::read(&provenance).expect("provenance")).expect("sidecar");
    assert_eq!(sidecar["profile"], "solo");
    assert!(
        sidecar["profile_path"]
            .as_str()
            .expect("profile_path")
            .ends_with("/profiles/solo.yaml")
    );
    assert_eq!(
        sidecar["digest"].as_str().expect("digest").len(),
        64,
        "the sidecar records the full digest"
    );
    let pieces = sidecar["pieces"].as_array().expect("pieces");
    assert_eq!(pieces.len(), 1);
    assert_eq!(pieces[0]["name"], "base");
    assert!(
        pieces[0]["path"]
            .as_str()
            .expect("path")
            .ends_with("/settings/base.json")
    );
    assert!(
        sidecar.get("keys").is_none(),
        "the contributor map arrives with the merge engine"
    );
}
