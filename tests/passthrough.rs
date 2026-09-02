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
        .bound_command()
        .args([os(Vec::new()), os(vec![0x66, 0x80, 0x6f]), "tail".into()])
        .status()
        .expect("wrapper");
    assert!(status.success());
    let argv = read_nul(&harness.record_dir().join("argv"));
    assert_eq!(argv[0], bytes(harness.child().as_os_str()));
    assert_eq!(
        &argv[3..],
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
    harness.initialize_companion_account();
    assert!(
        harness
            .command()
            .args([
                "--account".into(),
                "companion".into(),
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
    harness.initialize_token_in(
        &Path::new(&state).join("claude-session"),
        "companion",
        b"companion-token",
        b"companion-token",
    );
    assert!(
        harness
            .command()
            .env("XDG_STATE_HOME", &state)
            .args(["--account", "companion", "--profile", "work", "run"])
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

/// This terminal's own session directory is the child's configuration
/// directory, the account's directory is its credential store, and those are
/// the only keys besides the marker that the wrapper adds. A second
/// `CLAUDE_SESSION_*` key would mean wrapper state reached a program that
/// has no business reading it.
#[test]
fn a_selected_account_injects_its_config_directory_and_the_marker_alone() {
    let harness = Harness::new();
    harness.initialize_login("work");
    harness.initialize_companion_profile();
    assert!(
        harness
            .command()
            .args(["--account", "work", "--profile", "companion"])
            .status()
            .expect("wrapper")
            .success()
    );
    let environ = read_nul(&harness.record_dir().join("environ"));
    let sessions = harness.state().join("accounts/work/sessions");
    let config = harness.state().join("accounts/work/config");
    assert!(
        environ.windows(2).any(|pair| {
            pair[0] == b"CLAUDE_CONFIG_DIR"
                && pair[1].starts_with(bytes(sessions.as_os_str()))
                && pair[1] != bytes(sessions.as_os_str())
        }),
        "this terminal's session directory did not reach the child"
    );
    assert!(
        environ.windows(2).any(|pair| {
            pair[0] == b"CLAUDE_SECURESTORAGE_CONFIG_DIR" && pair[1] == bytes(config.as_os_str())
        }),
        "the account credential store did not reach the child"
    );
    assert!(
        !environ
            .iter()
            .any(|item| item == b"CLAUDE_CODE_PLUGIN_SEED_DIR"),
        "a launch with no seed tree names no seed to the child"
    );
    let internal: Vec<_> = environ
        .iter()
        .filter(|item| item.starts_with(b"CLAUDE_SESSION_"))
        .collect();
    assert_eq!(internal, [b"CLAUDE_SESSION_REENTRY"], "{internal:?}");
}

/// Slice 038 acceptance: a launch names the seed tree to the child, and drops
/// an inherited one whether or not it has a tree of its own.
///
/// The two halves are one test because the second is only meaningful against
/// the first: a wrapper that never set the variable would pass the drop
/// assertion by doing nothing at all.
#[test]
fn a_plugin_seed_reaches_the_child_and_displaces_an_inherited_one() {
    let harness = Harness::new();
    harness.initialize_login("work");
    harness.initialize_companion_profile();
    let seed = harness.write_plugin_seed(&["known_marketplaces.json"]);
    assert!(
        harness
            .command()
            .env(
                "CLAUDE_CODE_PLUGIN_SEED_DIR",
                harness.root().join("inherited")
            )
            .args(["--account", "work", "--profile", "companion"])
            .status()
            .expect("wrapper")
            .success()
    );
    let environ = read_nul(&harness.record_dir().join("environ"));
    let seen: Vec<_> = environ
        .windows(2)
        .filter(|pair| pair[0] == b"CLAUDE_CODE_PLUGIN_SEED_DIR")
        .map(|pair| pair[1].clone())
        .collect();
    assert_eq!(
        seen,
        [bytes(seed.as_os_str()).to_vec()],
        "the wrapper's own seed reaches the child, and only it"
    );
}

/// Slice 038 acceptance: an inherited seed is dropped even when the wrapper
/// supplies none, because it names a tree this launch did not choose.
#[test]
fn an_inherited_plugin_seed_is_dropped_without_one_of_our_own() {
    let harness = Harness::new();
    harness.initialize_login("work");
    harness.initialize_companion_profile();
    assert!(
        harness
            .command()
            .env(
                "CLAUDE_CODE_PLUGIN_SEED_DIR",
                harness.root().join("inherited")
            )
            .args(["--account", "work", "--profile", "companion"])
            .status()
            .expect("wrapper")
            .success()
    );
    let environ = read_nul(&harness.record_dir().join("environ"));
    assert!(
        !environ
            .iter()
            .any(|item| item == b"CLAUDE_CODE_PLUGIN_SEED_DIR"),
        "an inherited seed must not reach a launch that supplies none"
    );
}

/// A seed path is a path, so its bytes are the user's and not necessarily
/// UTF-8. Round-tripping one through a `String` would corrupt it.
#[test]
fn a_non_utf8_plugin_seed_path_reaches_the_child_intact() {
    use std::os::unix::ffi::OsStrExt as _;
    let harness = Harness::new();
    harness.initialize_login("work");
    harness.initialize_companion_profile();
    // The seed lives at a name the wrapper computes, so the invalid bytes go
    // into the XDG base above it, which is the only part a test can choose.
    let base = harness
        .root()
        .join(std::ffi::OsStr::from_bytes(b"data-\xff"));
    std::fs::create_dir_all(base.join("claude-session/plugin-seed")).expect("seed fixture");
    std::fs::create_dir_all(base.join("claude-session/assets/skills")).expect("asset fixture");
    std::fs::write(
        base.join("claude-session/plugin-seed/known_marketplaces.json"),
        b"{}\n",
    )
    .expect("seed fixture");
    assert!(
        harness
            .command()
            .env("XDG_DATA_HOME", &base)
            .args(["--account", "work", "--profile", "companion"])
            .status()
            .expect("wrapper")
            .success()
    );
    let environ = read_nul(&harness.record_dir().join("environ"));
    let expected = base.join("claude-session/plugin-seed");
    assert!(
        environ
            .windows(2)
            .any(|pair| pair[0] == b"CLAUDE_CODE_PLUGIN_SEED_DIR"
                && pair[1] == bytes(expected.as_os_str())),
        "the seed path reached the child with its bytes intact"
    );
}

#[test]
fn sentinel_preserves_child_suffix() {
    let harness = Harness::new();
    assert!(
        harness
            .bound_command()
            .args(["--", "--", "--verbose"])
            .status()
            .expect("wrapper")
            .success()
    );
    assert_eq!(
        &read_nul(&harness.record_dir().join("argv"))[3..],
        &[b"--".to_vec(), b"--verbose".to_vec()]
    );
}

/// The sentinel is unconditional, so a wrapper verb spelling behind it belongs
/// to the child. `--` is also the documented remedy for reaching a shadowed
/// child surface, which only works if the wrapper never looks past it.
#[test]
fn a_wrapper_verb_behind_the_sentinel_reaches_the_child() {
    for verb in [
        "completion",
        "config",
        "doctor",
        "help",
        "man",
        "profile",
        "version",
    ] {
        let harness = Harness::new();
        assert!(
            harness
                .bound_command()
                .args(["--", verb])
                .status()
                .expect("wrapper")
                .success()
        );
        assert_eq!(
            &read_nul(&harness.record_dir().join("argv"))[3..],
            &[verb.as_bytes().to_vec()],
            "the wrapper claimed `{verb}` from behind the sentinel"
        );
    }
}

/// Every verb the wrapper's own CLI surface documents is now implemented, so
/// the contract this pins is the general one slice 001 stated: a verb spelling
/// the parser does not declare stays the child's, unchanged. The fixtures are
/// real child verbs the wrapper deliberately never claims.
#[test]
fn unimplemented_verbs_reach_child() {
    for verb in ["mcp", "update", "agents"] {
        let harness = Harness::new();
        assert!(
            harness
                .bound_command()
                .arg(verb)
                .status()
                .expect("wrapper")
                .success()
        );
        assert_eq!(
            read_nul(&harness.record_dir().join("argv"))[3],
            verb.as_bytes(),
            "the wrapper claimed the undeclared spelling `{verb}`"
        );
    }
}

#[test]
fn leading_position_limits_wrapper_flags() {
    let harness = Harness::new();
    assert!(
        harness
            .bound_command()
            .args(["native", "--verbose"])
            .status()
            .expect("wrapper")
            .success()
    );
    assert_eq!(
        &read_nul(&harness.record_dir().join("argv"))[3..],
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
            .bound_command()
            .arg("--configg=x")
            .status()
            .expect("wrapper")
            .success()
    );
    assert_eq!(
        read_nul(&harness.record_dir().join("argv"))[3],
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
        .bound_command()
        .env("CS_TEST_EXIT", "73")
        .status()
        .expect("wrapper");
    assert_eq!(status.code(), Some(73));
    let harness = Harness::new();
    let status = harness
        .bound_command()
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
            .bound_command()
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
        .bound_command()
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
        .bound_command()
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
