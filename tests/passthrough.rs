#![allow(clippy::expect_used, clippy::pedantic, clippy::nursery)]

mod support;

use std::{ffi::OsString, os::unix::process::ExitStatusExt, path::Path, process::Stdio};
use support::{Harness, bytes, os, read_nul};

const PIECE: &str = r#"{"model":"sonnet"}"#;
const PROFILE: &str = "layers:\n  - base\n";

#[test]
fn golden_argv_preserves_bytes_order_count_and_empty_values() {
    let harness = Harness::new();
    let status = harness
        .command()
        .args([os(Vec::new()), os(vec![0x66, 0x80, 0x6f]), "tail".into()])
        .status()
        .expect("wrapper");
    assert!(status.success());
    let argv = read_nul(&harness.record_dir().join("argv"));
    assert_eq!(argv[0], bytes(harness.child().as_os_str()));
    assert_eq!(
        &argv[1..],
        &[Vec::new(), vec![0x66, 0x80, 0x6f], b"tail".to_vec()]
    );
}

/// The wrapper's own two tokens are a prefix and nothing more: the suffix that
/// follows them is the user's, unchanged in order, count, and bytes.
#[test]
fn golden_argv_prefixes_the_settings_pair_before_an_untouched_suffix() {
    let harness = Harness::new();
    harness.write_piece("base", PIECE);
    harness.write_profile("work", PROFILE);
    assert!(
        harness
            .command()
            .args([
                "--profile".into(),
                "work".into(),
                os(Vec::new()),
                os(vec![0x66, 0x80, 0x6f]),
                OsString::from("tail"),
            ])
            .status()
            .expect("wrapper")
            .success()
    );
    let argv = read_nul(&harness.record_dir().join("argv"));
    assert_eq!(argv[1], b"--settings");
    let store = harness.state().join("composed");
    assert!(
        argv[2].starts_with(bytes(store.as_os_str())) && argv[2].ends_with(b".json"),
        "the second token is not the composed entry: {:?}",
        String::from_utf8_lossy(&argv[2])
    );
    assert_eq!(
        &argv[3..],
        &[Vec::new(), vec![0x66, 0x80, 0x6f], b"tail".to_vec()]
    );
}

/// The settings path comes from the XDG bases, whose bytes are arbitrary, so it
/// travels as an OS string. A wrapper that rendered it as text would either
/// mangle this path or refuse to launch at all.
#[test]
fn a_non_utf8_state_home_reaches_the_child_byte_exact() {
    let harness = Harness::new();
    harness.write_piece("base", PIECE);
    harness.write_profile("work", PROFILE);
    let mut raw = bytes(harness.root().as_os_str()).to_vec();
    raw.extend_from_slice(&[b'/', b's', 0x80, b't']);
    let state = os(raw.clone());
    std::fs::create_dir_all(Path::new(&state)).expect("non-UTF-8 state base");
    assert!(
        harness
            .command()
            .env("XDG_STATE_HOME", &state)
            .args(["--profile", "work", "run"])
            .status()
            .expect("wrapper")
            .success()
    );
    let argv = read_nul(&harness.record_dir().join("argv"));
    let mut expected = raw;
    expected.extend_from_slice(b"/claude-session/composed/profile-work-");
    assert_eq!(argv[1], b"--settings");
    assert!(
        argv[2].starts_with(&expected),
        "the composed path lost its raw bytes: {:?}",
        String::from_utf8_lossy(&argv[2])
    );
    assert_eq!(argv[3], b"run");
}

/// The account's own directory is the child's, and it is the only key besides
/// the marker that the wrapper adds. A second `CLAUDE_SESSION_*` key would mean
/// wrapper state reached a program that has no business reading it.
#[test]
fn a_selected_account_injects_its_config_directory_and_the_marker_alone() {
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
    let environ = read_nul(&harness.record_dir().join("environ"));
    let config = harness.state().join("accounts/work/config");
    assert!(
        environ
            .windows(2)
            .any(|pair| pair[0] == b"CLAUDE_CONFIG_DIR" && pair[1] == bytes(config.as_os_str())),
        "the account configuration directory did not reach the child"
    );
    let internal: Vec<_> = environ
        .iter()
        .filter(|item| item.starts_with(b"CLAUDE_SESSION_"))
        .collect();
    assert_eq!(internal, [b"CLAUDE_SESSION_REENTRY"], "{internal:?}");
}

#[test]
fn sentinel_preserves_child_suffix() {
    let harness = Harness::new();
    assert!(
        harness
            .command()
            .args(["--", "--", "--verbose"])
            .status()
            .expect("wrapper")
            .success()
    );
    assert_eq!(
        &read_nul(&harness.record_dir().join("argv"))[1..],
        &[b"--".to_vec(), b"--verbose".to_vec()]
    );
}

/// The sentinel is unconditional, so a wrapper verb spelling behind it belongs
/// to the child. `--` is also the documented remedy for reaching a shadowed
/// child surface, which only works if the wrapper never looks past it.
#[test]
fn a_wrapper_verb_behind_the_sentinel_reaches_the_child() {
    for verb in ["completion", "doctor", "help", "man", "profile", "version"] {
        let harness = Harness::new();
        assert!(
            harness
                .command()
                .args(["--", verb])
                .status()
                .expect("wrapper")
                .success()
        );
        assert_eq!(
            &read_nul(&harness.record_dir().join("argv"))[1..],
            &[verb.as_bytes().to_vec()],
            "the wrapper claimed `{verb}` from behind the sentinel"
        );
    }
}

/// `config` is documented on the CLI surface but not yet built, so it is still
/// the child's. The single remaining spelling is named rather than looped over,
/// because a one-element loop reads as a list that happens to be short.
#[test]
fn unimplemented_verbs_reach_child() {
    let verb = "config";
    let harness = Harness::new();
    assert!(
        harness
            .command()
            .arg(verb)
            .status()
            .expect("wrapper")
            .success()
    );
    assert_eq!(
        read_nul(&harness.record_dir().join("argv"))[1],
        verb.as_bytes()
    );
}

#[test]
fn leading_position_limits_wrapper_flags() {
    let harness = Harness::new();
    assert!(
        harness
            .command()
            .args(["native", "--verbose"])
            .status()
            .expect("wrapper")
            .success()
    );
    assert_eq!(
        &read_nul(&harness.record_dir().join("argv"))[1..],
        &[b"native".to_vec(), b"--verbose".to_vec()]
    );
}

#[test]
fn malformed_wrapper_flag_is_usage_and_near_miss_forwards() {
    let harness = Harness::new();
    let output = harness.command().arg("--config").output().expect("wrapper");
    assert_eq!(output.status.code(), Some(64));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("error[Usage]"));
    let harness = Harness::new();
    assert!(
        harness
            .command()
            .arg("--configg=x")
            .status()
            .expect("wrapper")
            .success()
    );
    assert_eq!(
        read_nul(&harness.record_dir().join("argv"))[1],
        b"--configg=x"
    );
}

/// A regression check rather than a mapping test: the exec leaves one process,
/// so an exit code and a signal death are the child's own by construction. What
/// this rejects is a wrapper that reappears between them and translates either.
#[test]
fn child_exit_and_signal_status_are_preserved() {
    let harness = Harness::new();
    let status = harness
        .command()
        .env("CS_TEST_EXIT", "73")
        .status()
        .expect("wrapper");
    assert_eq!(status.code(), Some(73));
    let harness = Harness::new();
    let status = harness
        .command()
        .env("CS_TEST_ABORT", "1")
        .status()
        .expect("wrapper");
    assert_eq!(status.signal(), Some(6));
}

#[test]
fn child_environment_is_scrubbed_and_preserved() {
    let harness = Harness::new();
    assert!(
        harness
            .command()
            .env("KEEP_RAW", os(vec![0x66, 0x80]))
            .env("CLAUDE_SESSION_SECRET", "gone")
            .status()
            .expect("wrapper")
            .success()
    );
    let env = read_nul(&harness.record_dir().join("environ"));
    assert!(
        env.windows(2)
            .any(|pair| pair == [b"KEEP_RAW".to_vec(), vec![0x66, 0x80]])
    );
    assert!(!env.iter().any(|item| item == b"CLAUDE_SESSION_SECRET"));
    assert!(
        env.windows(2)
            .any(|pair| pair == [b"CLAUDE_SESSION_REENTRY".to_vec(), b"1".to_vec()])
    );
}

#[test]
fn passthrough_stdout_contains_only_child_bytes() {
    let harness = Harness::new();
    let output = harness
        .command()
        .env("CS_TEST_STDOUT", "child-only")
        .stderr(Stdio::piped())
        .output()
        .expect("wrapper");
    assert_eq!(output.stdout, b"child-only");
}

/// The end-to-end half of the flush contract: a real launch leaves a readable
/// log behind. That the flush happens before the exec rather than after it is a
/// question of order, which no observation of a replaced process can answer, so
/// `tests::the_flush_precedes_the_launch` owns it in the unit lane.
#[test]
fn a_signalled_run_still_leaves_a_complete_log() {
    let harness = Harness::new();
    let status = harness
        .command()
        .env("CS_TEST_ABORT", "1")
        .status()
        .expect("wrapper");
    assert_eq!(status.signal(), Some(6));
    let log = std::fs::read_to_string(
        harness
            .root()
            .join("state/claude-session/claude-session.log"),
    )
    .expect("UTF-8 structured log");
    assert!(
        log.contains("op=resolve_child"),
        "the sink was never flushed before the re-raise:\n{log}"
    );
}
