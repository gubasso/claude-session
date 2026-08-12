//! The generated configuration artifacts.
//!
//! A package integration test links the library target, so these call the
//! renderers directly rather than shelling out to cargo. That is the whole
//! reason the renderers live in the shipped crate and `xtask` is a thin CLI
//! over them.

#![allow(clippy::expect_used, clippy::pedantic, clippy::nursery)]

mod support;

use std::path::PathBuf;

use support::Harness;

fn repository(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

/// The freshness gate, proven in the test lane as well as the hook: a
/// checked-in artifact that no longer matches its type fails here.
#[test]
fn every_generated_artifact_matches_fresh_generator_output() {
    for (relative, expected) in claude_session::render::artifacts() {
        let path = repository(relative);
        let actual = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{relative} must exist: {error}"));
        assert_eq!(
            actual, expected,
            "{relative} is stale; run `cargo xtask gen-config`"
        );
        assert!(
            expected.ends_with('\n') && !expected.ends_with("\n\n"),
            "{relative} must end with exactly one newline"
        );
        assert!(!expected.contains('\r'), "{relative} must be LF-normalized");
    }
}

/// An example the program itself would reject is worse than none.
#[test]
fn the_generated_configuration_example_round_trips_through_the_real_loader() {
    let harness = Harness::new();
    let file = harness.root().join("generated.toml");
    std::fs::write(&file, claude_session::render::config_example_toml()).expect("example");

    let output = harness
        .companion_account_command()
        .args(["--config".into(), file.into_os_string()])
        .args(["config", "--json"])
        .output()
        .expect("config runs");
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let document: serde_json::Value = serde_json::from_slice(&output.stdout).expect("one document");
    // Every key is optional and every one is commented out, so the uncommented
    // body is a minimal valid configuration that sets nothing.
    for key in ["child_bin", "default_account", "default_profile"] {
        let entry = &document["configuration"][key];
        assert!(
            entry["source"] != "user-config",
            "{key} came from the example, so it is not commented out"
        );
    }
    let row = document["files"]
        .as_array()
        .expect("files")
        .iter()
        .find(|row| row["layer"] == "user-config")
        .expect("the user layer");
    assert_eq!(row["existed"], true, "the loader read the example");
}

#[test]
fn the_generated_profile_example_round_trips_through_the_profile_type() {
    let harness = Harness::new();
    let yaml = claude_session::render::profile_example_yaml();
    // The example names placeholder pieces, so the pieces it names are written
    // to prove composition rather than only parsing.
    harness.write_profile("example", &yaml);
    harness.write_piece(
        "replace-me",
        r#"{"model":"a","permissions":{"allow":["x"]}}"#,
    );
    harness.write_piece(
        "replace-me-too",
        r#"{"permissions":{"allow":["y"]},"hooks":{"PreToolUse":[{"matcher":"Bash"}]}}"#,
    );

    let listed = harness
        .command()
        .arg("profile")
        .output()
        .expect("profile runs");
    assert_eq!(listed.status.code(), Some(0));
    assert!(
        String::from_utf8_lossy(&listed.stdout).contains("example"),
        "the generated profile is not listed"
    );

    let launched = harness
        .companion_account_command()
        .args(["--profile", "example"])
        .output()
        .expect("wrapper");
    assert_eq!(
        launched.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&launched.stderr)
    );
}

#[test]
fn the_generated_schema_rejects_an_unknown_key() {
    let schema: serde_json::Value =
        serde_json::from_str(&claude_session::render::config_schema_json()).expect("valid JSON");
    assert_eq!(
        schema["additionalProperties"], false,
        "the schema must agree with deny_unknown_fields"
    );
    assert_eq!(schema["type"], "object");
    let properties = schema["properties"].as_object().expect("properties");
    for key in claude_session::schema::KEYS {
        let property = properties
            .get(key.name)
            .unwrap_or_else(|| panic!("the schema omits {}", key.name));
        assert_eq!(property["type"], key.type_name);
        assert_eq!(property["description"], key.description);
    }
    assert_eq!(properties.len(), claude_session::schema::KEYS.len());
}

/// The mechanism ADR-0013 names: a field with no description cannot be
/// rendered, so adding one without documenting it cannot pass review.
#[test]
fn every_generated_key_carries_a_non_empty_description() {
    for key in claude_session::schema::KEYS {
        assert!(
            !key.description.trim().is_empty(),
            "{} carries no description",
            key.name
        );
        assert!(
            !key.placeholder.trim().is_empty(),
            "{} carries no placeholder",
            key.name
        );
    }
}
