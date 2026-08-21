//! Witness recording, session liveness verdicts, and dead-session collection.
//!
//! Slice 033 acceptance lives here: a launch records the witness beside its
//! session directory, `session list` tells the three states apart, and
//! `session clean` removes exactly the provably dead after a confirmation
//! ([ADR-0110], [ADR-0111]).
//!
//! [ADR-0110]: ../docs/decisions/ADR-0110-record-the-terminal-witness-at-launch.md
//! [ADR-0111]: ../docs/decisions/ADR-0111-collect-only-the-provably-dead-session.md

#![allow(clippy::expect_used, clippy::pedantic, clippy::nursery)]

mod support;

use std::fs;
use std::path::{Path, PathBuf};

use support::{Harness, mode};

/// Returns the one namespace directory a launch created under an account.
fn only_namespace_dir(harness: &Harness, account: &str) -> PathBuf {
    let sessions = harness.state().join(format!("accounts/{account}/sessions"));
    let mut entries: Vec<_> = fs::read_dir(&sessions)
        .expect("namespace level")
        .map(|entry| entry.expect("entry").path())
        .filter(|path| path.is_dir())
        .collect();
    assert_eq!(entries.len(), 1, "one launch makes one namespace directory");
    entries.pop().expect("entry")
}

/// Returns the one session directory inside a namespace directory.
fn only_session_dir(namespace: &Path) -> PathBuf {
    let mut entries: Vec<_> = fs::read_dir(namespace)
        .expect("terminal level")
        .map(|entry| entry.expect("entry").path())
        .filter(|path| path.is_dir())
        .collect();
    assert_eq!(entries.len(), 1, "one launch makes one session directory");
    entries.pop().expect("entry")
}

/// Returns the witness path recorded beside one session directory.
fn witness_path(session: &Path) -> PathBuf {
    let terminal = session.file_name().expect("name").to_string_lossy();
    session
        .parent()
        .expect("namespace")
        .join(format!(".{terminal}.witness.json"))
}

/// Plants a session directory that is provably dead in this run's own scope.
///
/// The fabricated record copies the real launch's rung and namespace so it is
/// decidable wherever the suite runs — with or without a controlling terminal
/// — and is dead by construction: an absent device for the tty rung, a foreign
/// boot for the session-leader rung.
fn plant_dead_session(namespace: &Path, real_witness: &serde_json::Value, name: &str) -> PathBuf {
    let directory = namespace.join(name);
    fs::create_dir(&directory).expect("dead session directory");
    let ns = real_witness["namespace"].as_str().expect("namespace field");
    let mut dead = if real_witness["rung"] == "tty" {
        serde_json::json!({
            "version": 1,
            "rung": "tty",
            "device": "/dev/claude-session-rs-absent-fixture",
            "namespace": ns,
        })
    } else {
        serde_json::json!({
            "version": 1,
            "rung": "session-leader",
            "sid": 1,
            "started": 1,
            "namespace": ns,
            "boot": "a-boot-that-never-was",
        })
    };
    if real_witness["rung"] == "tty"
        && let Some(boot) = real_witness.get("boot")
    {
        dead["boot"] = boot.clone();
    }
    fs::write(
        namespace.join(format!(".{name}.witness.json")),
        serde_json::to_vec(&dead).expect("dead witness"),
    )
    .expect("dead witness file");
    directory
}

/// Reads a `session` subcommand's JSON document from a fresh invocation.
fn session_json(harness: &Harness, arguments: &[&str]) -> serde_json::Value {
    let output = harness
        .assert_command()
        .args(arguments)
        .output()
        .expect("session verb");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("session document")
}

/// Slice 033 acceptance: a launch records the witness beside its session
/// directory, and a second launch of the same terminal leaves an unchanged
/// record unwritten.
#[test]
fn a_launch_records_the_witness_and_rewrites_it_only_on_change() {
    use std::os::unix::fs::MetadataExt as _;
    let harness = Harness::new();
    assert!(
        harness.bound_command().status().expect("wrapper").success(),
        "the first launch runs"
    );
    let namespace = only_namespace_dir(&harness, "companion");
    let session = only_session_dir(&namespace);
    let witness = witness_path(&session);
    assert_eq!(mode(&witness), 0o600, "the record is private");
    let value: serde_json::Value =
        serde_json::from_slice(&fs::read(&witness).expect("witness")).expect("witness json");
    assert_eq!(value["version"], 1);
    assert!(
        value["rung"] == "tty" || value["rung"] == "session-leader",
        "{value}"
    );
    assert_eq!(
        value["namespace"].as_str(),
        namespace.file_name().expect("name").to_str(),
        "the record names the namespace directory it is filed under"
    );
    let inode = fs::metadata(&witness).expect("witness metadata").ino();
    assert!(
        harness.bound_command().status().expect("wrapper").success(),
        "the second launch runs"
    );
    assert_eq!(
        fs::metadata(&witness).expect("witness metadata").ino(),
        inode,
        "an unchanged record is not rewritten"
    );
}

/// Slice 033 acceptance: every session directory carries exactly one verdict,
/// and one without a readable record is unknown.
#[test]
fn session_list_tells_the_three_states_apart() {
    let harness = Harness::new();
    assert!(
        harness.bound_command().status().expect("wrapper").success(),
        "the launch runs"
    );
    let namespace = only_namespace_dir(&harness, "companion");
    let session = only_session_dir(&namespace);
    let real: serde_json::Value =
        serde_json::from_slice(&fs::read(witness_path(&session)).expect("witness"))
            .expect("witness json");
    plant_dead_session(&namespace, &real, "deadslot");
    fs::create_dir(namespace.join("unknownslot")).expect("markerless directory");
    let document = session_json(&harness, &["session", "list", "--json"]);
    let sessions = document["sessions"].as_array().expect("rows");
    assert_eq!(sessions.len(), 3, "{document}");
    let verdict = |terminal: &str| {
        sessions
            .iter()
            .find(|row| row["terminal"] == terminal)
            .unwrap_or_else(|| panic!("no row for {terminal}: {document}"))["verdict"]
            .clone()
    };
    assert_eq!(
        verdict(&session.file_name().expect("name").to_string_lossy()),
        "live",
        "the launch's own terminal still exists"
    );
    assert_eq!(verdict("deadslot"), "dead");
    assert_eq!(verdict("unknownslot"), "unknown");
    let human = harness
        .assert_command()
        .args(["session", "list"])
        .output()
        .expect("human list");
    let text = String::from_utf8_lossy(&human.stdout).into_owned();
    assert!(text.contains("Sessions"), "{text}");
    assert!(
        text.contains("session clean"),
        "a dead finding names the collector: {text}"
    );
}

/// Slice 033 acceptance: `session clean` removes every dead session directory
/// and nothing live or unknown.
#[test]
fn session_clean_removes_exactly_the_dead() {
    let harness = Harness::new();
    assert!(
        harness.bound_command().status().expect("wrapper").success(),
        "the launch runs"
    );
    let namespace = only_namespace_dir(&harness, "companion");
    let live = only_session_dir(&namespace);
    let real: serde_json::Value =
        serde_json::from_slice(&fs::read(witness_path(&live)).expect("witness"))
            .expect("witness json");
    let dead = plant_dead_session(&namespace, &real, "deadslot");
    let unknown = namespace.join("unknownslot");
    fs::create_dir(&unknown).expect("markerless directory");
    let document = session_json(&harness, &["session", "clean", "--yes", "--json"]);
    let removed = document["removed"].as_array().expect("removed rows");
    assert_eq!(removed.len(), 1, "{document}");
    assert_eq!(removed[0]["terminal"], "deadslot");
    assert_eq!(document["pruned_namespaces"], 0, "{document}");
    assert!(!dead.exists(), "the dead directory is gone");
    assert!(
        !namespace.join(".deadslot.witness.json").exists(),
        "its witness went with it"
    );
    assert!(live.exists(), "the live session stays");
    assert!(witness_path(&live).exists(), "and keeps its witness");
    assert!(
        unknown.exists(),
        "the unknown session is kept, never guessed"
    );
}

/// Slice 033 acceptance: a namespace directory is removed only once it is
/// empty — and one emptied by the collection goes, orphan records and all.
#[test]
fn an_emptied_namespace_directory_is_pruned() {
    let harness = Harness::new();
    assert!(
        harness.bound_command().status().expect("wrapper").success(),
        "the launch runs"
    );
    let namespace = only_namespace_dir(&harness, "companion");
    let real: serde_json::Value = serde_json::from_slice(
        &fs::read(witness_path(&only_session_dir(&namespace))).expect("witness"),
    )
    .expect("witness json");
    // A second account holding only a dead session, in this run's own scope.
    let other = harness
        .state()
        .join("accounts/other/sessions")
        .join(namespace.file_name().expect("name"));
    fs::create_dir_all(&other).expect("other namespace directory");
    plant_dead_session(&other, &real, "deadslot");
    let document = session_json(&harness, &["session", "clean", "--yes", "--json"]);
    assert_eq!(
        document["removed"].as_array().expect("rows").len(),
        1,
        "{document}"
    );
    assert_eq!(document["pruned_namespaces"], 1, "{document}");
    assert!(!other.exists(), "the emptied namespace directory is pruned");
    assert!(
        namespace.exists(),
        "the namespace still holding the live session stays"
    );
}

/// Slice 033 acceptance: with no controlling terminal and no `--yes`, `clean`
/// refuses before any side effect.
#[test]
fn clean_without_a_terminal_and_without_yes_refuses_first() {
    let harness = Harness::new();
    assert!(
        harness.bound_command().status().expect("wrapper").success(),
        "the launch runs"
    );
    let namespace = only_namespace_dir(&harness, "companion");
    let real: serde_json::Value = serde_json::from_slice(
        &fs::read(witness_path(&only_session_dir(&namespace))).expect("witness"),
    )
    .expect("witness json");
    let dead = plant_dead_session(&namespace, &real, "deadslot");
    let output = harness
        .detached_command(&["session", "clean"])
        .output()
        .expect("detached clean");
    assert_eq!(
        output.status.code(),
        Some(69),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("--yes"),
        "the refusal names the escape: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(dead.exists(), "nothing was removed");
}

/// Slice 033 acceptance: a declined prompt removes nothing and exits `0`.
#[test]
fn a_declined_prompt_removes_nothing_and_exits_zero() {
    use std::io::Write as _;
    let harness = Harness::new();
    assert!(
        harness.bound_command().status().expect("wrapper").success(),
        "the launch runs"
    );
    let namespace = only_namespace_dir(&harness, "companion");
    let real: serde_json::Value = serde_json::from_slice(
        &fs::read(witness_path(&only_session_dir(&namespace))).expect("witness"),
    )
    .expect("witness json");
    let dead = plant_dead_session(&namespace, &real, "deadslot");
    let mut child = harness
        .terminal_command("session clean")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("terminal clean");
    let mut stdin = child.stdin.take().expect("terminal input");
    stdin.write_all(b"n\n").expect("decline");
    stdin.flush().expect("decline");
    drop(stdin);
    let output = child.wait_with_output().expect("terminal clean");
    assert!(
        output.status.success(),
        "declining is an outcome, not an error: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    let text = String::from_utf8_lossy(&output.stdout).into_owned();
    assert!(
        text.contains("Remove them? [y/N]"),
        "the preview is part of the question: {text}"
    );
    assert!(
        text.contains("deadslot"),
        "the prompt names the path: {text}"
    );
    assert!(dead.exists(), "nothing was removed");
}

/// The sentinel still forces the spelling to the child: claiming the verb must
/// not cost the passthrough a token ([ADR-0002]).
///
/// [ADR-0002]: ../docs/decisions/ADR-0002-verbatim-argv-passthrough.md
#[test]
fn the_sentinel_still_forwards_the_session_spelling_to_the_child() {
    let harness = Harness::new();
    let mut command = harness.bound_command();
    command.args(["--", "session", "list"]);
    assert!(
        command.status().expect("wrapper").success(),
        "the passthrough runs"
    );
    let words: Vec<_> = support::read_nul(&harness.record_dir().join("argv"))
        .iter()
        .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
        .collect();
    assert_eq!(words.last().map(String::as_str), Some("list"), "{words:?}");
    assert_eq!(
        words.get(words.len().saturating_sub(2)).map(String::as_str),
        Some("session"),
        "{words:?}"
    );
}

/// A witness reached through a symbolic link is no record at all: the session
/// is judged unknown and never collected, whatever the link's target says
/// ([ADR-0061]).
///
/// [ADR-0061]: ../docs/decisions/ADR-0061-protect-storage-from-accidental-local-drift.md
#[test]
fn a_symlinked_witness_is_unknown_and_never_collected() {
    let harness = Harness::new();
    assert!(
        harness.bound_command().status().expect("wrapper").success(),
        "the launch runs"
    );
    let namespace = only_namespace_dir(&harness, "companion");
    let real: serde_json::Value = serde_json::from_slice(
        &fs::read(witness_path(&only_session_dir(&namespace))).expect("witness"),
    )
    .expect("witness json");
    plant_dead_session(&namespace, &real, "deadslot");
    // A directory whose witness is only a link to the dead record: followed,
    // it would spell a provably dead terminal.
    let linked = namespace.join("linkedslot");
    fs::create_dir(&linked).expect("linked session directory");
    std::os::unix::fs::symlink(
        namespace.join(".deadslot.witness.json"),
        namespace.join(".linkedslot.witness.json"),
    )
    .expect("witness link");
    let document = session_json(&harness, &["session", "list", "--json"]);
    let row = document["sessions"]
        .as_array()
        .expect("rows")
        .iter()
        .find(|row| row["terminal"] == "linkedslot")
        .unwrap_or_else(|| panic!("no row for linkedslot: {document}"))
        .clone();
    assert_eq!(row["verdict"], "unknown", "{document}");
    let document = session_json(&harness, &["session", "clean", "--yes", "--json"]);
    let removed = document["removed"].as_array().expect("removed rows");
    assert_eq!(removed.len(), 1, "{document}");
    assert_eq!(removed[0]["terminal"], "deadslot", "{document}");
    assert!(linked.exists(), "the linked slot is kept, never guessed");
}

/// A slot reborn while the confirmation waits is left standing: only a
/// verdict re-judged inside the removal's critical section is acted on
/// ([ADR-0111]).
///
/// [ADR-0111]: ../docs/decisions/ADR-0111-collect-only-the-provably-dead-session.md
#[test]
fn a_slot_reborn_during_the_prompt_survives_the_clean() {
    use std::io::{Read as _, Write as _};
    let harness = Harness::new();
    assert!(
        harness.bound_command().status().expect("wrapper").success(),
        "the launch runs"
    );
    let namespace = only_namespace_dir(&harness, "companion");
    let live = only_session_dir(&namespace);
    let real_bytes = fs::read(witness_path(&live)).expect("witness");
    let real: serde_json::Value = serde_json::from_slice(&real_bytes).expect("witness json");
    let dead = plant_dead_session(&namespace, &real, "deadslot");
    let mut child = harness
        .terminal_command("session clean")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("terminal clean");
    let mut stdout = child.stdout.take().expect("terminal output");
    let mut seen = Vec::new();
    let mut chunk = [0_u8; 1024];
    // The preview precedes the question, so once it is visible the survey has
    // already judged the slot dead.
    while !String::from_utf8_lossy(&seen).contains("Remove them? [y/N]") {
        let count = stdout.read(&mut chunk).expect("prompt bytes");
        assert!(count > 0, "the prompt never appeared: {seen:?}");
        seen.extend_from_slice(&chunk[..count]);
    }
    // The slot is reborn while the question waits: its witness now spells the
    // launch's own live terminal.
    fs::write(namespace.join(".deadslot.witness.json"), &real_bytes).expect("reborn witness");
    let mut stdin = child.stdin.take().expect("terminal input");
    stdin.write_all(b"y\n").expect("consent");
    stdin.flush().expect("consent");
    drop(stdin);
    let mut rest = Vec::new();
    stdout.read_to_end(&mut rest).expect("remaining output");
    assert!(
        child.wait().expect("terminal clean").success(),
        "{}",
        String::from_utf8_lossy(&rest)
    );
    assert!(dead.exists(), "the reborn slot is left standing");
    assert!(live.exists(), "the live session stays");
}

/// A widened witness is settled back to `0600` by the next launch even when
/// its bytes have not changed: the guard runs before the unchanged-bytes
/// fast path, so mode drift never rides the early return.
#[test]
fn an_unchanged_widened_witness_is_restored_on_relaunch() {
    use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
    let harness = Harness::new();
    assert!(
        harness.bound_command().status().expect("wrapper").success(),
        "the first launch runs"
    );
    let witness = witness_path(&only_session_dir(&only_namespace_dir(
        &harness,
        "companion",
    )));
    let bytes = fs::read(&witness).expect("witness");
    let inode = fs::metadata(&witness).expect("witness metadata").ino();
    fs::set_permissions(&witness, fs::Permissions::from_mode(0o644)).expect("widen");
    assert!(
        harness.bound_command().status().expect("wrapper").success(),
        "the second launch runs"
    );
    assert_eq!(mode(&witness), 0o600, "the drifted mode is corrected");
    assert_eq!(
        fs::metadata(&witness).expect("witness metadata").ino(),
        inode,
        "the unchanged record is not rewritten"
    );
    assert_eq!(
        fs::read(&witness).expect("witness"),
        bytes,
        "the bytes are untouched"
    );
}

/// The defect this fixes: naming a terminal by opening `/dev/tty` and asking
/// its name back yields the alias every process shares, so every pane of one
/// namespace was given a single directory named `tty`, and that directory
/// could never be judged gone. A launch inside a real pseudo-terminal must
/// name the pane it is in ([ADR-0102], [ADR-0110]).
#[test]
fn a_launch_under_a_pty_names_the_pane_it_runs_in() {
    let harness = Harness::new();
    // Constructing it is what initializes the companion account and profile;
    // the launch itself has to happen under the allocator below.
    let _ = harness.bound_command();
    let status = harness
        .terminal_command("--account companion --profile companion")
        .status()
        .expect("wrapper under a pseudo-terminal");
    assert!(status.success(), "the launch runs under a pty");
    let namespace = only_namespace_dir(&harness, "companion");
    let session = only_session_dir(&namespace);
    let witness: serde_json::Value =
        serde_json::from_slice(&fs::read(witness_path(&session)).expect("witness"))
            .expect("witness json");
    assert_eq!(witness["rung"], "tty", "{witness}");
    let device = witness["device"].as_str().expect("a recorded device");
    assert_ne!(
        device, "/dev/tty",
        "the alias names no pane and must never be recorded"
    );
    let index = device
        .strip_prefix("/dev/pts/")
        .unwrap_or_else(|| panic!("the pane's own device, not {device}"));
    assert!(
        index.chars().all(|character| character.is_ascii_digit()),
        "{device}"
    );
    assert_eq!(
        session.file_name().expect("name").to_string_lossy(),
        format!("pts-{index}"),
        "the directory is named after the pane"
    );
}

/// A record written before the fix names the alias, so it witnesses no pane:
/// it is kept as unknown and never collected, because what is inside belonged
/// to every terminal at once ([ADR-0111]).
#[test]
fn a_recorded_alias_is_unknown_and_never_collected() {
    let harness = Harness::new();
    let _ = harness.bound_command();
    assert!(
        harness
            .terminal_command("--account companion --profile companion")
            .status()
            .expect("wrapper under a pseudo-terminal")
            .success(),
        "the launch runs under a pty"
    );
    let namespace = only_namespace_dir(&harness, "companion");
    let scope = namespace.file_name().expect("name").to_string_lossy();
    let alias = namespace.join("tty");
    fs::create_dir(&alias).expect("alias session directory");
    fs::write(
        namespace.join(".tty.witness.json"),
        serde_json::to_vec(&serde_json::json!({
            "version": 1,
            "rung": "tty",
            "device": "/dev/tty",
            "namespace": scope,
        }))
        .expect("alias witness"),
    )
    .expect("alias witness file");
    let document = session_json(&harness, &["session", "list", "--json"]);
    let row = document["sessions"]
        .as_array()
        .expect("rows")
        .iter()
        .find(|row| row["terminal"] == "tty")
        .unwrap_or_else(|| panic!("no row for the alias: {document}"))
        .clone();
    assert_eq!(row["verdict"], "unknown", "{document}");
    assert_eq!(row["ground"], "alias", "{document}");
    // The record names no terminal, so it reports none: the key's absence is
    // what tells a caller that, rather than a device standing for every pane.
    assert!(row.get("names").is_none(), "{document}");
    // The pane the launch ran in closed with the allocator, so its own slot may
    // be collected here; the alias record is the one that must survive, and it
    // must survive because nothing about it can be proven.
    let collected = session_json(&harness, &["session", "clean", "--yes", "--json"]);
    assert!(
        !collected["removed"]
            .as_array()
            .expect("removed")
            .iter()
            .any(|row| row["terminal"] == "tty"),
        "{collected}"
    );
    assert!(alias.exists(), "the alias directory is kept");
}

/// Every row says why, in both forms: the machine document carries the ground
/// and the human report carries the sentence it stands for.
#[test]
fn every_verdict_reports_the_ground_it_stands_on() {
    let harness = Harness::new();
    assert!(
        harness.bound_command().status().expect("wrapper").success(),
        "the launch runs"
    );
    let namespace = only_namespace_dir(&harness, "companion");
    let session = only_session_dir(&namespace);
    let real: serde_json::Value =
        serde_json::from_slice(&fs::read(witness_path(&session)).expect("witness"))
            .expect("witness json");
    plant_dead_session(&namespace, &real, "deadslot");
    fs::create_dir(namespace.join("unknownslot")).expect("markerless directory");
    let document = session_json(&harness, &["session", "list", "--json"]);
    let sessions = document["sessions"].as_array().expect("rows");
    let ground = |terminal: &str| {
        sessions
            .iter()
            .find(|row| row["terminal"] == terminal)
            .unwrap_or_else(|| panic!("no row for {terminal}: {document}"))["ground"]
            .as_str()
            .unwrap_or_else(|| panic!("no ground for {terminal}: {document}"))
            .to_owned()
    };
    assert_eq!(ground("unknownslot"), "unrecorded", "{document}");
    let live = ground(&session.file_name().expect("name").to_string_lossy());
    assert!(
        ["device-present", "leader-running"].contains(&live.as_str()),
        "the launch's own rung grounds its liveness: {live}"
    );
    let dead = ground("deadslot");
    assert!(
        ["device-absent", "leader-foreign-boot"].contains(&dead.as_str()),
        "the planted fixture is dead by construction: {dead}"
    );
    let human = harness
        .assert_command()
        .args(["session", "list"])
        .output()
        .expect("human list");
    let text = String::from_utf8_lossy(&human.stdout).into_owned();
    assert!(text.contains("[dead]"), "the verdict leads the row: {text}");
    assert!(
        text.contains("Collectable:"),
        "the row says why it is collectable: {text}"
    );
    assert!(
        text.contains("carries no record of the terminal"),
        "an unrecorded session says so in words: {text}"
    );
    assert!(
        text.contains("session clean"),
        "a dead finding names the collector: {text}"
    );
}

/// The `current` column is terminal-scoped rather than session-scoped: it
/// marks every session directory this run's own terminal owns, which is more
/// than one row once that terminal has launched under more than one account.
/// Every leg runs inside one pane, because a fresh pane per leg would be a
/// different terminal and prove nothing.
#[test]
fn every_session_directory_of_this_terminal_is_current() {
    let harness = Harness::new();
    harness.initialize_companion_profile();
    harness.initialize_token("companion", b"companion-token", b"companion-token");
    harness.initialize_token("second", b"second-token", b"second-token");
    // The listing redirects to a file so the pane's echo is not mixed into the
    // document, and the human leg is read back the same way.
    let status = harness
        .terminal_sequence(&[
            "--account companion --profile companion",
            "--account second --profile companion",
            "session list --json > sessions.json",
            "session list > sessions.txt",
        ])
        .status()
        .expect("wrapper under a pseudo-terminal");
    assert!(status.success(), "every leg runs under one pty");
    let document: serde_json::Value =
        serde_json::from_slice(&fs::read(harness.root().join("sessions.json")).expect("document"))
            .expect("session document");
    let sessions = document["sessions"].as_array().expect("rows");
    let current: Vec<&serde_json::Value> = sessions
        .iter()
        .filter(|row| row["current"] == serde_json::Value::Bool(true))
        .collect();
    assert_eq!(
        current.len(),
        2,
        "one pane under two accounts is two current rows: {document}"
    );
    let mut accounts: Vec<&str> = current
        .iter()
        .map(|row| row["account"].as_str().expect("account"))
        .collect();
    accounts.sort_unstable();
    assert_eq!(accounts, ["companion", "second"], "{document}");
    // The two rows are one terminal, which is what makes them both current.
    let terminals: std::collections::BTreeSet<&str> = current
        .iter()
        .map(|row| row["terminal"].as_str().expect("terminal"))
        .collect();
    assert_eq!(terminals.len(), 1, "one pane, one terminal: {document}");
    let human = fs::read_to_string(harness.root().join("sessions.txt")).expect("human report");
    assert_eq!(
        human.matches("— this terminal").count(),
        2,
        "the human report marks the same rows: {human}"
    );
}

/// Another terminal's directory inside this run's own namespace is not
/// current: the comparison is on the terminal, so sharing a namespace with the
/// reporting run is not enough. The run's own row is asserted current in the
/// same breath, because a comparison that marked nothing would satisfy the
/// negative half on its own.
#[test]
fn a_session_of_another_terminal_is_not_current() {
    let harness = Harness::new();
    assert!(
        harness.bound_command().status().expect("wrapper").success(),
        "the launch runs"
    );
    let namespace = only_namespace_dir(&harness, "companion");
    let session = only_session_dir(&namespace);
    let real: serde_json::Value =
        serde_json::from_slice(&fs::read(witness_path(&session)).expect("witness"))
            .expect("witness json");
    plant_dead_session(&namespace, &real, "otherslot");
    let document = session_json(&harness, &["session", "list", "--json"]);
    let row = document["sessions"]
        .as_array()
        .expect("rows")
        .iter()
        .find(|row| row["terminal"] == "otherslot")
        .unwrap_or_else(|| panic!("no row for the planted slot: {document}"));
    assert_eq!(row["current"], false, "{document}");
    // The other half: the run's own directory is current, which is what makes
    // the negative above a discrimination rather than a blanket false.
    let own = session
        .file_name()
        .expect("name")
        .to_string_lossy()
        .into_owned();
    let mine = document["sessions"]
        .as_array()
        .expect("rows")
        .iter()
        .find(|row| row["terminal"] == own)
        .unwrap_or_else(|| panic!("no row for this run's own session: {document}"));
    assert_eq!(mine["current"], true, "{document}");
    // Every row carries the key, so a caller reads it rather than inferring it
    // from an absence.
    for row in document["sessions"].as_array().expect("rows") {
        assert!(row["current"].is_boolean(), "{document}");
    }
}
