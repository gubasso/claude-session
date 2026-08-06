#![allow(clippy::expect_used, clippy::pedantic, clippy::nursery)]

mod support;

use std::{os::unix::process::ExitStatusExt, process::Stdio};
use support::{Harness, os, read_nul};

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
    assert_eq!(
        &argv[1..],
        &[Vec::new(), vec![0x66, 0x80, 0x6f], b"tail".to_vec()]
    );
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

#[test]
fn unimplemented_verbs_reach_child() {
    for verb in [
        "account",
        "config",
        "profile",
        "doctor",
        "completion",
        "man",
    ] {
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
