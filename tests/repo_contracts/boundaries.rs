//! The two architectural rules ADR-0074 left outside clippy: the dependency
//! direction below `domain`, and the isolation of development tooling from the
//! shipped crate. Both are module-graph and manifest facts, which no lint can
//! express.

use crate::violation::{Violation, assert_clean};
use crate::{read, tree};

const MANIFEST: &str = "Cargo.toml";
const DOMAIN: &str = "src/domain";
const SOURCE: &str = "src";

/// Crates that exist to serve development tooling and must never enter the
/// shipped dependency graph.
const TOOLING: &[&str] = &["schemars", "toml_edit", "claude-session"];

/// Floors, so a renamed directory fails the gate rather than emptying its
/// input. The shell predecessor concatenated `find` output and grepped it: an
/// empty string matches nothing, so a vanished `src/domain` read as clean.
const SOURCE_FLOOR: usize = 25;
const DOMAIN_FLOOR: usize = 4;
const SENTINELS: &[&str] = &["src/main.rs", "src/domain/argv.rs"];

fn imports_adapter_or_service(text: &str) -> bool {
    text.contains("crate::adapters") || text.contains("crate::services")
}

/// A whole-word scan, so `xtask::Generator` is a reference and `my_xtask` is
/// not. This is the hazard a text scan carries and must be written to tolerate.
fn imports_xtask(text: &str) -> bool {
    let bytes = text.as_bytes();
    let word = b"xtask";
    bytes
        .windows(word.len())
        .enumerate()
        .any(|(index, window)| {
            if window != word {
                return false;
            }
            let before = index.checked_sub(1).map(|i| bytes[i]);
            let after = bytes.get(index + word.len()).copied();
            !before.is_some_and(is_word_byte) && !after.is_some_and(is_word_byte)
        })
}

fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// The keys of the shipped `[dependencies]` table.
///
/// Parsed rather than line-matched: the shell predecessor sliced the manifest
/// with awk and would have missed a `[dependencies.foo]` subtable entirely.
/// `toml` is already a shipped dependency, so this costs no admission.
fn shipped_dependencies(manifest: &str) -> Vec<String> {
    let document: toml::Value = toml::from_str(manifest)
        .unwrap_or_else(|err| panic!("{MANIFEST} is required by the repository gates: {err}"));
    document
        .get("dependencies")
        .and_then(toml::Value::as_table)
        .map(|table| table.keys().cloned().collect())
        .unwrap_or_default()
}

fn manifest_violations(manifest: &str) -> Vec<Violation> {
    shipped_dependencies(manifest)
        .into_iter()
        .filter(|name| TOOLING.contains(&name.as_str()))
        .map(|name| {
            Violation::whole(
                MANIFEST,
                format!("development-tooling crate {name} found in shipped dependencies"),
            )
        })
        .collect()
}

#[test]
fn the_domain_layer_imports_no_adapter_or_service() {
    let mut found = Vec::new();
    for path in tree::files(DOMAIN, &[".rs"]) {
        if imports_adapter_or_service(&read(&path)) {
            found.push(Violation::whole(
                path.as_str(),
                "imports from adapters or services are forbidden below domain",
            ));
        }
    }
    assert_clean(&found);
}

#[test]
fn nothing_under_src_imports_xtask() {
    let mut found = Vec::new();
    for path in tree::files(SOURCE, &[".rs"]) {
        if imports_xtask(&read(&path)) {
            found.push(Violation::whole(
                path.as_str(),
                "imports from xtask are forbidden in the shipped crate",
            ));
        }
    }
    assert_clean(&found);
}

#[test]
fn the_shipped_manifest_lists_no_tooling_crate() {
    assert_clean(&manifest_violations(&read(MANIFEST)));
}

#[test]
fn the_source_walk_found_the_crate() {
    let source = tree::files(SOURCE, &[".rs"]);
    let domain = tree::files(DOMAIN, &[".rs"]);
    assert!(
        source.len() >= SOURCE_FLOOR,
        "found {} sources under {SOURCE}, expected at least {SOURCE_FLOOR}",
        source.len()
    );
    assert!(
        domain.len() >= DOMAIN_FLOOR,
        "found {} sources under {DOMAIN}, expected at least {DOMAIN_FLOOR}",
        domain.len()
    );
    for sentinel in SENTINELS {
        assert!(
            source.iter().any(|path| path == sentinel),
            "{sentinel} was not recovered by the walk"
        );
    }
}

// The tests below prove the gate can go red, from literals rather than files.

#[test]
fn a_domain_import_is_not_a_violation() {
    assert!(!imports_adapter_or_service("use crate::domain::Thing;"));
}

#[test]
fn an_adapter_import_in_domain_is_a_violation() {
    assert!(imports_adapter_or_service(
        "use crate::adapters::FileSystem;"
    ));
    assert!(imports_adapter_or_service("use crate::services::Child;"));
}

#[test]
fn an_xtask_import_is_a_violation() {
    assert!(imports_xtask("use xtask::Generator;"));
    assert!(imports_xtask("xtask"));
    assert!(imports_xtask("run xtask now"));
}

#[test]
fn the_word_xtask_inside_an_identifier_is_not_a_violation() {
    assert!(!imports_xtask("use crate::domain::Thing;"));
    assert!(!imports_xtask("let my_xtask = 1;"));
    assert!(!imports_xtask("xtaskish"));
    assert!(!imports_xtask("prefixtask"));
}

#[test]
fn a_tooling_crate_in_dependencies_is_a_violation() {
    let manifest = "[dependencies]\nschemars = \"1\"\ntoml = \"0.9\"\n";
    let found = manifest_violations(manifest);
    assert_eq!(found.len(), 1, "{}", crate::violation::render(&found));
    assert!(found[0].to_string().contains("schemars"));
}

#[test]
fn a_tooling_crate_as_a_subtable_is_a_violation() {
    let manifest = "[dependencies.toml_edit]\nversion = \"1\"\n";
    let found = manifest_violations(manifest);
    assert_eq!(found.len(), 1, "{}", crate::violation::render(&found));
    assert!(found[0].to_string().contains("toml_edit"));
}

#[test]
fn a_tooling_crate_in_dev_dependencies_is_not_a_violation() {
    let manifest = "[dependencies]\ntoml = \"0.9\"\n\n[dev-dependencies]\nschemars = \"1\"\n";
    assert_clean(&manifest_violations(manifest));
}
