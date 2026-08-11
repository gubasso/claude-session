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

/// Detects a real dependency on the tooling member, not a mention of it.
///
/// Comments are stripped first and only the three forms that create a
/// dependency are matched — `use xtask`, `extern crate xtask`, and a path
/// beginning `xtask::`. This module's own prose names the member, as do the
/// ADR link paths in the shipped crate, and a bare substring scan would report
/// every one of them. The word-boundary check on the left stays, so `my_xtask`
/// is still not a match.
fn imports_xtask(text: &str) -> bool {
    text.lines()
        .map(|line| line.split("//").next().unwrap_or(""))
        .any(|code| {
            ["use xtask", "extern crate xtask", "xtask::"]
                .iter()
                .any(|form| contains_word_start(code, form))
        })
}

/// Reports whether `needle` occurs with no word byte immediately before it.
fn contains_word_start(text: &str, needle: &str) -> bool {
    let bytes = text.as_bytes();
    text.match_indices(needle).any(|(index, _)| {
        !index
            .checked_sub(1)
            .map(|before| bytes[before])
            .is_some_and(is_word_byte)
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
    assert!(imports_xtask("extern crate xtask;"));
    assert!(imports_xtask("let value = xtask::render();"));
    assert!(imports_xtask("    use xtask::thing::Nested;"));
}

/// The rule is about depending on the member, not about naming it. Once the
/// member exists, the shipped crate documents it and links its ADR, so a scan
/// that flagged a mention would flag the documentation that explains the rule.
#[test]
fn naming_xtask_without_depending_on_it_is_not_a_violation() {
    assert!(!imports_xtask("use crate::domain::Thing;"));
    assert!(!imports_xtask("let my_xtask = 1;"));
    assert!(!imports_xtask("xtaskish"));
    assert!(!imports_xtask("prefixtask"));
    assert!(!imports_xtask("// the xtask member hosts the generator"));
    assert!(!imports_xtask(
        "//! see ADR-0014-xtask-workspace-for-dev-tooling.md"
    ));
    assert!(!imports_xtask("let path = \"xtask/src/main.rs\";"));
    assert!(!imports_xtask("use crate::my_xtask::Thing;"));
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

/// The library target is the one-way door `xtask` reaches through, so it must
/// stay narrow: a `lib.rs` that grew into a re-export of every private module
/// would turn a development-tooling seam into a public API the binary never
/// promised (ADR-0014).
#[test]
fn the_library_exports_only_the_artifact_model() {
    let text = read("src/lib.rs");
    let exported: Vec<&str> = text
        .lines()
        .filter(|line| line.trim_start().starts_with("pub mod "))
        .map(|line| {
            line.trim()
                .trim_start_matches("pub mod ")
                .trim_end_matches(';')
        })
        .collect();
    assert_eq!(
        exported,
        ["schema", "render"],
        "the library target exports more than the artifact model"
    );
}

/// The tooling member depends on the shipped crate and never the reverse, and
/// it is never published.
#[test]
fn the_tooling_member_is_unpublished_and_depends_inward() {
    let manifest = read("xtask/Cargo.toml");
    let document: toml::Value = toml::from_str(&manifest).unwrap_or_else(|err| {
        panic!("xtask/Cargo.toml is required by the repository gates: {err}")
    });
    assert_eq!(
        document["package"]["publish"].as_bool(),
        Some(false),
        "the tooling member must never be published"
    );
    assert!(
        document["dependencies"]["claude-session"]
            .get("path")
            .is_some(),
        "the tooling member depends on the shipped crate by path"
    );
}

/// The shipped package excludes the tooling member and the cargo alias, so a
/// published crate carries neither.
#[test]
fn the_shipped_package_excludes_the_tooling_member() {
    let manifest = read(MANIFEST);
    let document: toml::Value = toml::from_str(&manifest)
        .unwrap_or_else(|err| panic!("{MANIFEST} is required by the repository gates: {err}"));
    let excluded: Vec<&str> = document["package"]["exclude"]
        .as_array()
        .expect("the package carries an exclude list")
        .iter()
        .filter_map(toml::Value::as_str)
        .collect();
    for path in ["/xtask", "/.cargo"] {
        assert!(
            excluded.contains(&path),
            "the package must exclude {path}; a denylist entry anchors one path"
        );
    }
}
