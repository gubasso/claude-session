#![allow(clippy::expect_used, clippy::pedantic, clippy::nursery)]

//! The credential-redaction property.
//!
//! The output rules bind every human line, every prompt, every `--json`
//! document, every log record, every diagnostic, and the diagnostic `where`
//! clause. That is a set that grows with every verb, so asserting it surface by
//! surface tests the surfaces someone remembered. This drives one account
//! through its whole lifecycle with a distinctive token and sweeps every byte
//! the wrapper wrote after each step.
//!
//! A scanner that matches nothing reports success, so the integrity proof below
//! is as load-bearing as the sweep: the token must be provably live in the
//! process, the scanner must be shown to go red, and the one permitted
//! secret-derived value must be shown present rather than swept away.

mod support;

use std::{fs, path::Path};
use support::{Harness, fingerprint};

/// Distinctive enough that a match is never a coincidence, and shaped like a
/// real credential so nothing rejects it before it reaches the surfaces.
const SENTINEL: &[u8] = b"sk-cs-redaction-sentinel-9f3a2b71c4d8";

/// The one path allowed to contain the token, relative to the state namespace.
const ALLOWED: &str = "accounts/work/oauth-token";

/// Returns every file under `root` whose bytes contain `needle`.
///
/// Byte-level and never through a `String`: a lossy render could hide a hit
/// behind a replacement character, which is exactly the failure a redaction
/// test must not have.
fn scan_tree(root: &Path, needle: &[u8], allowed: &Path) -> Vec<String> {
    let mut hits = Vec::new();
    let Ok(entries) = fs::read_dir(root) else {
        return hits;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_symlink() {
            continue;
        }
        if kind.is_dir() {
            hits.extend(scan_tree(&path, needle, allowed));
        } else if path != allowed && contains(&fs::read(&path).unwrap_or_default(), needle) {
            hits.push(path.display().to_string());
        }
    }
    hits
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

fn assert_absent(label: &str, haystack: &[u8]) {
    assert!(
        !contains(haystack, SENTINEL),
        "{label} leaked the whole token"
    );
    // A "safe prefix" is not safe: an eight-byte head of a credential is still
    // credential material, and a renderer that truncated one would pass the
    // check above.
    assert!(
        !contains(haystack, &SENTINEL[..8]),
        "{label} leaked a prefix of the token"
    );
}

/// The property, over the whole lifecycle in one fixture.
#[test]
fn no_surface_ever_emits_credential_material() {
    let harness = Harness::new();
    harness.initialize_companion_profile();
    let allowed = harness.state().join(ALLOWED);
    let sentinel = String::from_utf8(SENTINEL.to_vec()).expect("utf-8 sentinel");

    // Every step that could hold a live secret, including two that fail: a
    // failing probe produces an error document, a diagnostic, and a `where`
    // clause while the process is holding the token.
    let mut steps: Vec<(&str, Vec<u8>, Vec<u8>)> = Vec::new();
    let run = |label: &'static str, arguments: &[&str], stdin: Option<&str>, probe_exit| {
        let mut command = harness.assert_command();
        command.args(arguments);
        if let Some(exit) = probe_exit {
            command.env("CS_TEST_PROBE_EXIT", exit);
        }
        if let Some(input) = stdin {
            command.write_stdin(input.to_owned());
        }
        let output = command.output().expect("wrapper");
        (label, output.stdout, output.stderr)
    };

    steps.push(run(
        "login --token --stdin --verbose",
        &[
            "account",
            "login",
            "work",
            "--token",
            "--stdin",
            "--verbose",
            "--verbose",
            "--verbose",
        ],
        Some(&format!("{sentinel}\n")),
        None,
    ));
    steps.push(run("status", &["account", "status", "work"], None, None));
    steps.push(run(
        "status --json",
        &["account", "status", "work", "--json"],
        None,
        None,
    ));
    steps.push(run(
        "list --json",
        &["account", "list", "--json"],
        None,
        None,
    ));
    steps.push(run("doctor --json", &["doctor", "--json"], None, None));
    steps.push(run(
        "launch",
        &["--account", "work", "--profile", "companion", "--", "run"],
        None,
        None,
    ));
    // The failing legs. Each one renders an error while a secret is live.
    steps.push(run(
        "status with a failing probe",
        &["account", "status", "work", "--json"],
        None,
        Some("1"),
    ));
    steps.push(run(
        "rotation refused by the child",
        &["account", "login", "work", "--token", "--stdin"],
        Some(&format!("{sentinel}\n")),
        Some("1"),
    ));

    // The integrity proof, first: a green run must have been capable of red.
    assert_eq!(
        fs::read(&allowed).expect("the token file"),
        SENTINEL,
        "the token must actually be stored, or this test proves nothing"
    );
    // Every probe appends what it saw, so the record is the token repeated once
    // per probe. Asserting the shape rather than a count keeps this from
    // breaking when a step that happens to probe is added above.
    let probed = fs::read(harness.record_dir().join("probe-token")).expect("probe record");
    assert!(
        !probed.is_empty() && probed.chunks(SENTINEL.len()).all(|chunk| chunk == SENTINEL),
        "the child must actually have received the token, or it was never live"
    );
    let mut doctored = b"prefix ".to_vec();
    doctored.extend_from_slice(SENTINEL);
    assert!(
        contains(&doctored, SENTINEL),
        "the scanner must report a hit on a buffer that does hold the token"
    );
    assert!(
        !scan_tree(
            harness.state().as_path(),
            SENTINEL,
            Path::new("/definitely/not/the/token")
        )
        .is_empty(),
        "the tree scan must report the token file when it is not excluded"
    );

    // The sweep.
    for (label, stdout, stderr) in &steps {
        assert_absent(&format!("{label} standard output"), stdout);
        assert_absent(&format!("{label} standard error"), stderr);
    }
    let leaks = scan_tree(harness.state().as_path(), SENTINEL, &allowed);
    assert!(
        leaks.is_empty(),
        "the token reached files that must not hold it: {leaks:?}"
    );
    let argv = fs::read(harness.record_dir().join("argv")).unwrap_or_default();
    assert_absent("the child argument vector", &argv);

    // The one permitted exception, asserted present so a future sweep cannot
    // quietly delete it: `sha256[..8]` of a wrapper-owned token.
    let print = fingerprint(SENTINEL);
    let metadata =
        fs::read(harness.state().join("accounts/work/auth-mode.json")).expect("metadata file");
    assert!(
        contains(&metadata, print.as_bytes()),
        "the metadata must carry the fingerprint the rotation recorded"
    );
    let report = steps
        .iter()
        .find(|(label, _, _)| *label == "status --json")
        .expect("the status step ran");
    assert!(
        contains(&report.1, print.as_bytes()),
        "status reports the fingerprint, which is the permitted secret-derived value"
    );
}
