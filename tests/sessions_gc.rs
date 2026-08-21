//! Agent identity, session liveness verdicts, and session collection.
//!
//! A session is one running coding agent: a launch records the agent it is
//! about to become, `session list` tells live from garbage, and `session clean`
//! removes every directory it cannot prove is live ([ADR-0112], [ADR-0113]).
//!
//! [ADR-0112]: ../docs/decisions/ADR-0112-keep-only-the-session-proven-live.md
//! [ADR-0113]: ../docs/decisions/ADR-0113-key-a-session-to-its-running-agent.md

#![allow(clippy::expect_used, clippy::pedantic, clippy::nursery)]

mod support;

use std::fs;
use std::path::{Path, PathBuf};

use support::Harness;

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
        .expect("session level")
        .map(|entry| entry.expect("entry").path())
        .filter(|path| path.is_dir())
        .collect();
    assert_eq!(entries.len(), 1, "one live launch leaves one directory");
    entries.pop().expect("entry")
}

/// Returns the witness path recorded beside one session directory.
fn witness_path(session: &Path) -> PathBuf {
    let name = session.file_name().expect("name").to_string_lossy();
    session
        .parent()
        .expect("namespace")
        .join(format!(".{name}.witness.json"))
}

/// Reads the record a real launch left, which every fixture below copies its
/// namespace and boot from so it is judged in scope wherever the suite runs.
fn recorded(namespace: &Path) -> serde_json::Value {
    serde_json::from_slice(&fs::read(witness_path(&only_session_dir(namespace))).expect("witness"))
        .expect("witness json")
}

/// Reads one process's start time the way the wrapper does.
fn started(pid: u32) -> u64 {
    let text = fs::read_to_string(format!("/proc/{pid}/stat")).expect("stat");
    text.rsplit_once(')')
        .expect("comm")
        .1
        .split_whitespace()
        .nth(19)
        .expect("field 22")
        .parse()
        .expect("ticks")
}

/// Reads one process's run state, field 3 of its `stat`, the way the wrapper
/// does. `Z` is a process that has exited and has not been reaped.
fn state(pid: u32) -> String {
    let text = fs::read_to_string(format!("/proc/{pid}/stat")).expect("stat");
    text.rsplit_once(')')
        .expect("comm")
        .1
        .split_whitespace()
        .next()
        .expect("field 3")
        .to_owned()
}

/// Plants a session directory carrying a fabricated agent record.
fn plant(namespace: &Path, real: &serde_json::Value, name: &str, pid: u32, ticks: u64) -> PathBuf {
    let directory = namespace.join(name);
    fs::create_dir(&directory).expect("session directory");
    fs::write(
        namespace.join(format!(".{name}.witness.json")),
        serde_json::to_vec(&serde_json::json!({
            "version": 2,
            "pid": pid,
            "started": ticks,
            "namespace": real["namespace"].as_str().expect("namespace field"),
            "boot": real["boot"].as_str().expect("boot field"),
        }))
        .expect("witness"),
    )
    .expect("witness file");
    directory
}

/// Plants a session whose agent is running: this test process, which is.
fn plant_live(namespace: &Path, real: &serde_json::Value, name: &str) -> PathBuf {
    let pid = std::process::id();
    plant(namespace, real, name, pid, started(pid))
}

/// Plants a session whose agent has exited.
///
/// The identifier is this process's with a start time no process of it has, so
/// the fixture is dead by the rule that tells a reissued identifier from the
/// one the record witnessed, and needs no pid that might be reused mid-test.
fn plant_dead(namespace: &Path, real: &serde_json::Value, name: &str) -> PathBuf {
    plant(namespace, real, name, std::process::id(), 1)
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

/// Returns one row of a `session list` document by its session name.
fn row(document: &serde_json::Value, session: &str) -> serde_json::Value {
    document["sessions"]
        .as_array()
        .expect("rows")
        .iter()
        .find(|row| row["session"] == session)
        .unwrap_or_else(|| panic!("no row for {session}: {document}"))
        .clone()
}

/// Returns the shared peer registry a launch linked this session's `sessions`
/// name to, which is where the child writes its registrations.
fn registry(session: &Path) -> PathBuf {
    fs::read_link(session.join("sessions")).expect("the registry link a launch declares")
}

/// Plants the registration the child writes when it starts a session.
///
/// The field set is the child's, reduced to what the report reads: the process
/// the session runs as, the start time that proves whose registration it is,
/// and the name a person knows it by.
fn register(registry: &Path, pid: u32, ticks: u64, name: &str) {
    fs::write(
        registry.join(format!("{pid}.json")),
        serde_json::to_vec(&serde_json::json!({
            "pid": pid,
            "sessionId": "0f9a1c33-6f1e-4c21-9f3f-b0a2d4e6c810",
            "procStart": ticks.to_string(),
            "name": name,
            "nameSource": "derived",
        }))
        .expect("registration"),
    )
    .expect("registration file");
}

/// Returns the human `session list` report.
fn session_text(harness: &Harness) -> String {
    let output = harness
        .assert_command()
        .args(["session", "list"])
        .output()
        .expect("human list");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// Returns the row of the human report carrying one subject.
fn text_row(text: &str, subject: &str) -> String {
    text.lines()
        .find(|line| line.split_whitespace().any(|cell| cell == subject))
        .unwrap_or_else(|| panic!("no row for {subject}: {text}"))
        .to_owned()
}

/// Slice 036 acceptance: a launch records the agent it is about to become,
/// named by the process it will run as and the time that process started.
#[test]
fn a_launch_records_the_witness_naming_its_own_agent() {
    use std::os::unix::fs::MetadataExt as _;
    let harness = Harness::new();
    assert!(
        harness.bound_command().status().expect("wrapper").success(),
        "the launch runs"
    );
    let namespace = only_namespace_dir(&harness, "companion");
    let session = only_session_dir(&namespace);
    let witness = witness_path(&session);
    assert_eq!(
        fs::metadata(&witness).expect("witness metadata").mode() & 0o777,
        0o600,
        "the record is private"
    );
    let value: serde_json::Value =
        serde_json::from_slice(&fs::read(&witness).expect("witness")).expect("witness json");
    assert_eq!(value["version"], 2);
    let pid = value["pid"].as_u64().expect("pid");
    let ticks = value["started"].as_u64().expect("started");
    assert_eq!(
        session.file_name().expect("name").to_string_lossy(),
        format!("agent-{pid}-{ticks:x}"),
        "the directory spells the agent its record names"
    );
    // The criterion is that the record names the process the wrapper became,
    // which naming consistency alone cannot show: a wrapper recording any other
    // process would spell the directory from the same pair and agree with
    // itself. The child reports its own identity across the exec, so this is
    // the comparison that holds the wrapper to the right process.
    let reported = fs::read_to_string(harness.record_dir().join("agent")).expect("agent record");
    let (child_pid, child_ticks) = reported.trim().split_once(' ').expect("agent identity");
    assert_eq!(
        (child_pid.to_owned(), child_ticks.to_owned()),
        (pid.to_string(), ticks.to_string()),
        "the record names the process the wrapper exec'd into, not another"
    );
    assert_eq!(
        value["namespace"].as_str(),
        namespace.file_name().expect("name").to_str(),
        "the record names the namespace directory it is filed under"
    );
    assert!(
        value["boot"].as_str().is_some_and(|it| !it.is_empty()),
        "a start time counts ticks from a boot the record has to name: {value}"
    );
}

/// Slice 036 acceptance: a session is one agent run, so a second launch is a
/// second session rather than a reuse of the first one's directory.
#[test]
fn a_second_launch_is_a_second_session() {
    let harness = Harness::new();
    assert!(
        harness.bound_command().status().expect("wrapper").success(),
        "the first launch runs"
    );
    let namespace = only_namespace_dir(&harness, "companion");
    let first = only_session_dir_named_agent(&namespace);
    // Kept out of the second launch's reach, so this asserts the naming rather
    // than the sweep: a copy under a name no launch can issue is left alone by
    // the collector only if it is judged, so it is planted as a live agent.
    let real = recorded(&namespace);
    plant_live(&namespace, &real, "keepsake");
    assert!(
        harness.bound_command().status().expect("wrapper").success(),
        "the second launch runs"
    );
    let second = only_session_dir_named_agent(&namespace);
    assert_ne!(
        first, second,
        "a second agent must not inherit the first one's state"
    );
}

/// Slice 036 acceptance: a launch collects the sessions whose agents have
/// exited, so one directory per run does not accumulate between collections.
#[test]
fn a_launch_collects_the_sessions_whose_agents_exited() {
    let harness = Harness::new();
    assert!(
        harness.bound_command().status().expect("wrapper").success(),
        "the first launch runs"
    );
    let namespace = only_namespace_dir(&harness, "companion");
    let real = recorded(&namespace);
    let live = plant_live(&namespace, &real, "keepsake");
    let dead = plant_dead(&namespace, &real, "deadslot");
    let stray = namespace.join("strayslot");
    fs::create_dir(&stray).expect("markerless directory");
    let first = only_session_dir_named_agent(&namespace);
    assert!(
        harness.bound_command().status().expect("wrapper").success(),
        "the second launch runs"
    );
    assert!(!first.exists(), "the first launch's session went with it");
    assert!(!dead.exists(), "so did the planted dead one");
    assert!(live.exists(), "a running agent's session is never swept");
    assert!(
        stray.exists(),
        "a launch sweeps only the provably dead, never what it could not decide"
    );
}

/// Returns the one `agent-` directory a launch made, ignoring planted names.
fn only_session_dir_named_agent(namespace: &Path) -> PathBuf {
    let mut entries: Vec<_> = fs::read_dir(namespace)
        .expect("session level")
        .map(|entry| entry.expect("entry").path())
        .filter(|path| {
            path.is_dir()
                && path
                    .file_name()
                    .and_then(|it| it.to_str())
                    .is_some_and(|it| it.starts_with("agent-"))
        })
        .collect();
    assert_eq!(entries.len(), 1, "one launch makes one session directory");
    entries.pop().expect("entry")
}

/// Slice 036 acceptance: every session directory carries exactly one verdict,
/// and only one whose agent this run can prove is running is `live`.
#[test]
fn session_list_tells_the_three_states_apart() {
    let harness = Harness::new();
    assert!(
        harness.bound_command().status().expect("wrapper").success(),
        "the launch runs"
    );
    let namespace = only_namespace_dir(&harness, "companion");
    let real = recorded(&namespace);
    plant_live(&namespace, &real, "liveslot");
    plant_dead(&namespace, &real, "deadslot");
    fs::create_dir(namespace.join("strayslot")).expect("markerless directory");
    let document = session_json(&harness, &["session", "list", "--json"]);
    assert_eq!(row(&document, "liveslot")["verdict"], "live", "{document}");
    assert_eq!(row(&document, "deadslot")["verdict"], "dead", "{document}");
    assert_eq!(
        row(&document, "strayslot")["verdict"],
        "unknown",
        "{document}"
    );
    assert!(
        row(&document, "strayslot").get("pid").is_none(),
        "a directory with no record names no process: {document}"
    );
    let human = harness
        .assert_command()
        .args(["session", "list"])
        .output()
        .expect("human list");
    let text = String::from_utf8_lossy(&human.stdout).into_owned();
    for token in ["[live]", "[dead]", "[unknown]"] {
        assert!(text.contains(token), "{token} is missing: {text}");
    }
    // One aligned row per session, and nothing else between the heading and
    // the summary: a list is read by scanning rather than by reading.
    for line in text.lines().filter(|line| line.contains("[live]")) {
        assert!(
            line.contains("liveslot") && line.contains("running"),
            "the row carries its subject and its reason: {line}"
        );
    }
}

/// Slice 036 acceptance: a record written when a session meant a terminal is
/// not read, so the directory it named is unaccounted for and collectable.
#[test]
fn a_terminal_keyed_session_is_not_read_and_is_collected() {
    let harness = Harness::new();
    assert!(
        harness.bound_command().status().expect("wrapper").success(),
        "the launch runs"
    );
    let namespace = only_namespace_dir(&harness, "companion");
    let scope = namespace.file_name().expect("name").to_string_lossy();
    let legacy = namespace.join("pts-3");
    fs::create_dir(&legacy).expect("legacy session directory");
    fs::write(
        namespace.join(".pts-3.witness.json"),
        serde_json::to_vec(&serde_json::json!({
            "version": 1,
            "rung": "tty",
            "device": "/dev/pts/3",
            "namespace": scope,
        }))
        .expect("legacy witness"),
    )
    .expect("legacy witness file");
    let document = session_json(&harness, &["session", "list", "--json"]);
    let legacy_row = row(&document, "pts-3");
    assert_eq!(legacy_row["verdict"], "unknown", "{document}");
    assert_eq!(legacy_row["ground"], "unrecorded", "{document}");
    session_json(&harness, &["session", "clean", "--yes", "--json"]);
    assert!(
        !legacy.exists(),
        "the terminal-keyed directory is collected"
    );
    assert!(
        !namespace.join(".pts-3.witness.json").exists(),
        "its record went with it"
    );
}

/// An agent that exited but has not been reaped is over, so its directory is
/// garbage. The kernel goes on listing such a process and answers `kill(pid, 0)`
/// for it exactly as for a running one, so only its run state tells them apart.
#[test]
fn an_exited_agent_awaiting_its_reaper_is_not_live() {
    let harness = Harness::new();
    assert!(
        harness.bound_command().status().expect("wrapper").success(),
        "the launch runs"
    );
    let namespace = only_namespace_dir(&harness, "companion");
    let real = recorded(&namespace);
    // Never waited on, so the exited child stays listed for this test's whole
    // length: an unreaped child is what makes the state reachable at all.
    let mut exited = std::process::Command::new("true")
        .spawn()
        .expect("a process that exits at once");
    let pid = exited.id();
    let ticks = started(pid);
    for _ in 0..600 {
        if state(pid) == "Z" {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert_eq!(state(pid), "Z", "the fixture needs an unreaped process");
    plant(&namespace, &real, "reapedslot", pid, ticks);
    let document = session_json(&harness, &["session", "list", "--json"]);
    assert_eq!(
        row(&document, "reapedslot")["verdict"],
        "dead",
        "an agent that exited is not live because nobody reaped it: {document}"
    );
    assert_eq!(
        row(&document, "reapedslot")["ground"],
        "gone",
        "and the reason it gives is that the process is gone: {document}"
    );
    let _ = exited.wait();
}

/// Slice 036 acceptance: a command is never an agent, so the row it marks as
/// the reader's own is the agent it is running inside.
#[test]
fn the_reader_row_marks_the_agent_it_runs_inside() {
    let harness = Harness::new();
    assert!(
        harness.bound_command().status().expect("wrapper").success(),
        "the launch runs"
    );
    let namespace = only_namespace_dir(&harness, "companion");
    let real = recorded(&namespace);
    // The test process is an ancestor of the `session list` it spawns, so a
    // session naming it is the one that command is running inside.
    plant_live(&namespace, &real, "hostslot");
    // A live agent the reader does not descend from. It has to be a child
    // rather than any running process: everything on the machine descends
    // from process 1, so process 1 would be an ancestor and mark the row.
    let mut other = std::process::Command::new("sleep")
        .arg("30")
        .spawn()
        .expect("a process the reader does not descend from");
    let other_pid = other.id();
    plant(
        &namespace,
        &real,
        "otherslot",
        other_pid,
        started(other_pid),
    );
    let document = session_json(&harness, &["session", "list", "--json"]);
    assert_eq!(row(&document, "hostslot")["verdict"], "live", "{document}");
    assert_eq!(row(&document, "hostslot")["current"], true, "{document}");
    assert_eq!(row(&document, "otherslot")["verdict"], "live", "{document}");
    assert_eq!(
        row(&document, "otherslot")["current"],
        false,
        "a running agent the reader is not inside is not theirs: {document}"
    );
    let _ = other.kill();
    let _ = other.wait();
}

/// Slice 035 acceptance: `session clean` removes every session directory this
/// run cannot prove is live, and nothing else.
#[test]
fn session_clean_removes_everything_not_proven_live() {
    let harness = Harness::new();
    assert!(
        harness.bound_command().status().expect("wrapper").success(),
        "the launch runs"
    );
    let namespace = only_namespace_dir(&harness, "companion");
    let real = recorded(&namespace);
    let live = plant_live(&namespace, &real, "liveslot");
    let dead = plant_dead(&namespace, &real, "deadslot");
    let stray = namespace.join("strayslot");
    fs::create_dir(&stray).expect("markerless directory");
    let document = session_json(&harness, &["session", "clean", "--yes", "--json"]);
    let mut taken: Vec<&str> = document["removed"]
        .as_array()
        .expect("removed rows")
        .iter()
        .map(|row| row["session"].as_str().expect("session"))
        .collect();
    taken.sort_unstable();
    assert!(taken.contains(&"deadslot"), "{document}");
    assert!(taken.contains(&"strayslot"), "{document}");
    assert!(!dead.exists(), "the dead session is gone");
    assert!(
        !namespace.join(".deadslot.witness.json").exists(),
        "its record went with it"
    );
    assert!(
        !stray.exists(),
        "a directory carrying no record is garbage, not a question left open"
    );
    assert!(live.exists(), "the running agent's session stays");
    assert!(witness_path(&live).exists(), "and keeps its record");
}

/// Slice 035 acceptance: a witness reached through a symbolic link is no
/// record at all ([ADR-0061]), so the link neither speaks for the directory
/// nor saves it.
///
/// [ADR-0061]: ../docs/decisions/ADR-0061-protect-storage-from-accidental-local-drift.md
#[test]
fn a_symlinked_witness_speaks_for_nothing_and_saves_nothing() {
    let harness = Harness::new();
    assert!(
        harness.bound_command().status().expect("wrapper").success(),
        "the launch runs"
    );
    let namespace = only_namespace_dir(&harness, "companion");
    let real = recorded(&namespace);
    plant_live(&namespace, &real, "liveslot");
    // A directory whose record is only a link to a live one: followed, it
    // would spell a running agent and keep the directory standing.
    let linked = namespace.join("linkedslot");
    fs::create_dir(&linked).expect("linked session directory");
    std::os::unix::fs::symlink(
        namespace.join(".liveslot.witness.json"),
        namespace.join(".linkedslot.witness.json"),
    )
    .expect("witness link");
    let document = session_json(&harness, &["session", "list", "--json"]);
    assert_eq!(
        row(&document, "linkedslot")["verdict"],
        "unknown",
        "{document}"
    );
    assert_eq!(
        row(&document, "linkedslot")["ground"],
        "unrecorded",
        "the link is not read, so the directory carries no record: {document}"
    );
    session_json(&harness, &["session", "clean", "--yes", "--json"]);
    assert!(!linked.exists(), "an unaccounted directory is garbage");
}

/// Slice 033 acceptance: every row says why, in both forms — the document
/// carries the ground and the report carries the phrase it stands for.
#[test]
fn every_verdict_reports_the_ground_it_stands_on() {
    let harness = Harness::new();
    assert!(
        harness.bound_command().status().expect("wrapper").success(),
        "the launch runs"
    );
    let namespace = only_namespace_dir(&harness, "companion");
    let real = recorded(&namespace);
    plant_live(&namespace, &real, "liveslot");
    plant_dead(&namespace, &real, "deadslot");
    fs::create_dir(namespace.join("strayslot")).expect("markerless directory");
    let document = session_json(&harness, &["session", "list", "--json"]);
    assert_eq!(
        row(&document, "liveslot")["ground"],
        "running",
        "{document}"
    );
    assert_eq!(row(&document, "deadslot")["ground"], "gone", "{document}");
    assert_eq!(
        row(&document, "strayslot")["ground"],
        "unrecorded",
        "{document}"
    );
    let human = harness
        .assert_command()
        .args(["session", "list"])
        .output()
        .expect("human list");
    let text = String::from_utf8_lossy(&human.stdout).into_owned();
    for phrase in ["running", "agent exited", "no record of what it was"] {
        assert!(text.contains(phrase), "{phrase} is missing: {text}");
    }
    assert!(
        text.contains("claude-session session clean"),
        "the collector is named whole, never wrapped: {text}"
    );
}

/// Slice 033 acceptance: a namespace directory is removed only once nothing
/// but orphan records is left inside it.
#[test]
fn an_emptied_namespace_directory_is_pruned() {
    let harness = Harness::new();
    assert!(
        harness.bound_command().status().expect("wrapper").success(),
        "the launch runs"
    );
    let namespace = only_namespace_dir(&harness, "companion");
    let real = recorded(&namespace);
    plant_dead(&namespace, &real, "deadslot");
    // An orphan record: one whose directory a crash between the two removals
    // could have left behind. It witnesses nothing and keeps nothing alive.
    fs::write(
        namespace.join(".ghost.witness.json"),
        serde_json::to_vec(&serde_json::json!({"version": 2})).expect("orphan"),
    )
    .expect("orphan record");
    session_json(&harness, &["session", "clean", "--yes", "--json"]);
    assert!(
        !namespace.exists(),
        "nothing meaningful was left, so the namespace went too"
    );
}

/// Slice 033 acceptance: with no controlling terminal and no `--yes`, `clean`
/// refuses before any side effect.
///
/// Run detached, and it has to be: the prompt addresses `/dev/tty` rather than
/// standard input, so a run that inherits one asks the developer's own
/// terminal and waits there. `setsid` is what makes "no controlling terminal"
/// a property of the fixture instead of a property of where the suite happens
/// to be started from.
#[test]
fn clean_without_a_terminal_and_without_yes_refuses_first() {
    let harness = Harness::new();
    assert!(
        harness.bound_command().status().expect("wrapper").success(),
        "the launch runs"
    );
    let namespace = only_namespace_dir(&harness, "companion");
    let real = recorded(&namespace);
    let dead = plant_dead(&namespace, &real, "deadslot");
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

/// Slice 033 acceptance: a declined prompt removes nothing and exits `0`, and
/// the preview it declined groups by what collecting each row costs.
#[test]
fn a_declined_prompt_removes_nothing_and_exits_zero() {
    use std::io::Write as _;
    let harness = Harness::new();
    assert!(
        harness.bound_command().status().expect("wrapper").success(),
        "the launch runs"
    );
    let namespace = only_namespace_dir(&harness, "companion");
    let real = recorded(&namespace);
    let dead = plant_dead(&namespace, &real, "deadslot");
    let stray = namespace.join("strayslot");
    fs::create_dir(&stray).expect("markerless directory");
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
    for path in ["deadslot", "strayslot"] {
        assert!(text.contains(path), "the prompt names {path}: {text}");
    }
    for clause in ["whose agent has exited", "this run cannot account for"] {
        assert!(
            text.contains(clause),
            "the preview groups by what collecting costs: {clause} missing from {text}"
        );
    }
    assert!(dead.exists(), "nothing was removed");
    assert!(stray.exists(), "nothing was removed either");
}

/// Slice 037 acceptance: a row names its session the way its reader does.
#[test]
fn a_session_is_named_as_the_child_registered_it() {
    let harness = Harness::new();
    assert!(
        harness.bound_command().status().expect("wrapper").success(),
        "the launch runs"
    );
    let namespace = only_namespace_dir(&harness, "companion");
    let real = recorded(&namespace);
    let registry = registry(&only_session_dir_named_agent(&namespace));
    let live = plant_live(&namespace, &real, "liveslot");
    let pid = std::process::id();
    register(&registry, pid, started(pid), "claude-session-53");
    let document = session_json(&harness, &["session", "list", "--json"]);
    assert_eq!(
        row(&document, "liveslot")["name"],
        "claude-session-53",
        "the document carries the name and the directory both: {document}"
    );
    let text = session_text(&harness);
    let named = text_row(&text, "claude-session-53");
    assert!(
        named.contains("[live]") && !named.contains("liveslot"),
        "the row names the session as the child does: {named}"
    );
    assert!(live.exists(), "the report changed nothing about the tree");
}

/// Slice 037 acceptance: a process identifier is reused within one boot, so a
/// registration must not lend a live agent's name to the session that ran
/// under the same number and has exited.
#[test]
fn a_registration_of_another_start_time_names_nothing() {
    let harness = Harness::new();
    assert!(
        harness.bound_command().status().expect("wrapper").success(),
        "the launch runs"
    );
    let namespace = only_namespace_dir(&harness, "companion");
    let real = recorded(&namespace);
    let registry = registry(&only_session_dir_named_agent(&namespace));
    // The dead fixture is this process's identifier at a start time no process
    // of it has, which is exactly the reissue the pair defends against.
    plant_dead(&namespace, &real, "deadslot");
    let pid = std::process::id();
    register(&registry, pid, started(pid), "claude-session-53");
    let document = session_json(&harness, &["session", "list", "--json"]);
    assert!(
        row(&document, "deadslot").get("name").is_none(),
        "a registration the witness does not match names nothing: {document}"
    );
    let text = session_text(&harness);
    assert!(
        !text.contains("claude-session-53"),
        "and no row borrows it: {text}"
    );
    assert!(
        text_row(&text, "deadslot").contains("[dead]"),
        "the session keeps its directory name: {text}"
    );
}

/// Slice 037 acceptance: a session no registration accounts for is named by
/// its directory, which is the only name anybody has for it.
#[test]
fn an_unregistered_session_is_named_by_its_directory() {
    let harness = Harness::new();
    assert!(
        harness.bound_command().status().expect("wrapper").success(),
        "the launch runs"
    );
    let namespace = only_namespace_dir(&harness, "companion");
    let real = recorded(&namespace);
    plant_live(&namespace, &real, "liveslot");
    let document = session_json(&harness, &["session", "list", "--json"]);
    assert!(
        row(&document, "liveslot").get("name").is_none(),
        "{document}"
    );
    assert!(
        text_row(&session_text(&harness), "liveslot").contains("[live]"),
        "the row is still there, named by its directory"
    );
}

/// Slice 037 acceptance: the report is text the wrapper lays out itself, so a
/// name that could move a column or forge a status word is refused rather than
/// rendered.
#[test]
fn a_registered_name_holding_a_control_character_is_refused() {
    let harness = Harness::new();
    assert!(
        harness.bound_command().status().expect("wrapper").success(),
        "the launch runs"
    );
    let namespace = only_namespace_dir(&harness, "companion");
    let real = recorded(&namespace);
    let registry = registry(&only_session_dir_named_agent(&namespace));
    plant_live(&namespace, &real, "liveslot");
    let pid = std::process::id();
    register(&registry, pid, started(pid), "\u{1b}[31m[live]\u{1b}[0m");
    let document = session_json(&harness, &["session", "list", "--json"]);
    assert!(
        row(&document, "liveslot").get("name").is_none(),
        "{document}"
    );
    let text = session_text(&harness);
    assert!(
        !text.contains('\u{1b}'),
        "no escape byte reaches the report: {text:?}"
    );
    assert!(text_row(&text, "liveslot").contains("[live]"), "{text}");
}

/// Slice 037 acceptance: the rows are one table, laid out from the rows alone.
/// A column that answered to the terminal would make these bytes unpinnable.
#[test]
fn the_report_lays_its_rows_out_as_one_table() {
    let harness = Harness::new();
    assert!(
        harness.bound_command().status().expect("wrapper").success(),
        "the launch runs"
    );
    let namespace = only_namespace_dir(&harness, "companion");
    let real = recorded(&namespace);
    let registry = registry(&only_session_dir_named_agent(&namespace));
    plant_live(&namespace, &real, "liveslot");
    plant_dead(&namespace, &real, "deadslot");
    let pid = std::process::id();
    register(&registry, pid, started(pid), "claude-session-53");
    let text = session_text(&harness);
    let mut lines = text.lines();
    assert_eq!(lines.next(), Some("Sessions"), "{text}");
    assert_eq!(lines.next(), Some(""), "{text}");
    let header = lines.next().expect("the column names").to_owned();
    for column in ["status", "session", "account", "why"] {
        assert!(header.contains(column), "{header}");
    }
    let rule = lines.next().expect("the rule under them");
    assert!(
        rule.trim().chars().all(|c| c == '\u{2500}' || c == ' '),
        "the rule is a rule: {rule:?}"
    );
    // Each column's rule is that column's full width, so the rule is as wide
    // as the widest line the table can produce and no line overruns it.
    let widest = text
        .lines()
        .skip(2)
        .take_while(|line| !line.trim().is_empty())
        .map(|line| line.chars().count())
        .max()
        .expect("a table");
    assert_eq!(rule.chars().count(), widest, "the rule spans every column");
    // Every column starts at one offset, which is what makes the report
    // scannable and what a terminal-driven layout would break.
    let at = |line: &str, column: &str| line.find(column).expect("a column");
    let named = text_row(&text, "claude-session-53");
    let unnamed = text_row(&text, "deadslot");
    assert_eq!(
        at(&named, "claude-session-53"),
        at(&unnamed, "deadslot"),
        "the session column is one column: {text}"
    );
    assert_eq!(
        at(&named, "companion"),
        at(&unnamed, "companion"),
        "so is the account column: {text}"
    );
}
