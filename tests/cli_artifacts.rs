#![allow(clippy::expect_used, clippy::pedantic, clippy::nursery)]

//! Generated artifacts: shell completions and the man page.
//!
//! Every test here reads the compiled binary's standard output as bytes. None
//! of these verbs may reach the child, so each asserts the recording stub was
//! never invoked.

mod support;

use support::Harness;

/// The full shell set `clap_complete` supports, which `cli-surface.md#help`
/// makes the generator's fact rather than this project's, paired with how each
/// generator spells a declared long option.
///
/// The spelling differs per shell — `fish` emits `-l account` where the others
/// emit `--account` — so a single expected token would either miss `fish` or
/// weaken to something that does not prove the flag reached the script at all.
const SHELLS: [(&str, &str); 5] = [
    ("bash", "--account"),
    ("elvish", "--account"),
    ("fish", "-l account"),
    ("powershell", "--account"),
    ("zsh", "--account"),
];

/// Verbs whose parser nodes the artifacts must describe.
const IMPLEMENTED_VERBS: [&str; 9] = [
    "account",
    "completion",
    "config",
    "doctor",
    "help",
    "man",
    "profile",
    "session",
    "version",
];

/// Undoes roff's hyphen escaping so a spelling can be searched for as written.
///
/// Without this every flag assertion below would pass vacuously: `clap_mangen`
/// emits `--verbose` as `\-\-verbose`, so a raw search finds nothing whether the
/// spelling is in the page or not.
fn unescape_roff(page: &str) -> String {
    page.replace("\\-", "-")
}

fn completion(harness: &Harness, shell: &str) -> String {
    let output = harness
        .assert_command()
        .args(["completion", shell])
        .output()
        .expect("completion");
    assert!(
        output.status.success(),
        "completion {shell} exited {:?}",
        output.status.code()
    );
    assert!(output.stderr.is_empty(), "completion {shell} wrote stderr");
    assert!(
        !harness.record_dir().join("argv").exists(),
        "completion {shell} spawned the child"
    );
    String::from_utf8(output.stdout).expect("utf-8 completion script")
}

fn man_page(harness: &Harness) -> String {
    let output = harness.assert_command().arg("man").output().expect("man");
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    assert!(
        !harness.record_dir().join("argv").exists(),
        "man spawned the child"
    );
    String::from_utf8(output.stdout).expect("utf-8 roff")
}

/// Acceptance: a documented shell yields nonempty completions from the parser.
///
/// The flag assertions name wrapper-owned spellings the parser declares rather
/// than any canned string, so a generator emitting a valid but empty script
/// fails here instead of passing on length alone.
#[test]
fn completions_cover_every_documented_shell() {
    let harness = Harness::new();
    for (shell, option) in SHELLS {
        let script = completion(&harness, shell);
        assert!(!script.trim().is_empty(), "{shell} script was empty");
        assert!(
            script.contains("claude-session-rs") || script.contains("claude__session__rs"),
            "{shell} script never names the binary"
        );
        assert!(
            script.contains(option),
            "the {shell} script lacks the parser-declared option {option}"
        );
    }

    // The shell is a closed `ValueEnum`, so an unrecognized one is the parser's
    // own `Usage` and stdout stays clean for the caller's pipe.
    let rejected = Harness::new();
    let output = rejected
        .assert_command()
        .args(["completion", "nushell"])
        .output()
        .expect("unknown shell");
    assert_eq!(output.status.code(), Some(64));
    assert!(output.stdout.is_empty());

    // A bare verb is malformed and earns the node's own help as a diagnostic.
    let bare = Harness::new();
    let output = bare
        .assert_command()
        .arg("completion")
        .output()
        .expect("bare completion");
    assert_eq!(output.status.code(), Some(64));
    assert!(output.stdout.is_empty());
    let diagnostic = String::from_utf8(output.stderr).expect("utf-8 diagnostic");
    assert!(diagnostic.contains("Usage: claude-session-rs completion"));
    for (shell, _) in SHELLS {
        assert!(diagnostic.contains(shell), "the diagnostic hides {shell}");
    }
}

/// Acceptance: artifacts carry the whole implemented grammar.
///
/// Presence is asserted against the page's subcommand naming rather than the
/// completion scripts, because `config` and `profile` are also the spellings of
/// two wrapper flags — a scan for the bare token would match `--config` and
/// prove nothing about the verb. Proving it once at the page is sound: help,
/// completions, and the page read one `Command` tree, so a verb absent from the
/// tree is absent from all three.
#[test]
fn artifacts_carry_the_account_grammar_and_no_unimplemented_verb() {
    let harness = Harness::new();
    for (shell, account_flag) in SHELLS {
        let script = completion(&harness, shell);
        // The namespace's whole implemented subcommand set. Both generators
        // read one parser tree, so a spelling missing here means the tree lost
        // it rather than that the generator did.
        for token in ["account", "login", "list", "status", "remove"] {
            assert!(
                script.contains(token),
                "the {shell} script omits the account grammar token {token}"
            );
        }
        for token in ["session", "clean"] {
            assert!(
                script.contains(token),
                "the {shell} script omits the session grammar token {token}"
            );
        }
        // The verb-level flags, in this shell's own spelling — `fish` writes
        // `-l token` where the others write `--token`, which is the same
        // difference the account flag above already encodes.
        let long = |name: &str| {
            if account_flag.starts_with("-l ") {
                format!("-l {name}")
            } else {
                format!("--{name}")
            }
        };
        for name in ["token", "stdin", "minted-at", "yes"] {
            let spelling = long(name);
            assert!(
                script.contains(&spelling),
                "the {shell} script omits the account flag {spelling}"
            );
        }
    }

    let page = unescape_roff(&man_page(&harness));
    for verb in IMPLEMENTED_VERBS {
        assert!(
            page.contains(&format!("claude-session-rs-{verb}")),
            "the page omits the {verb} verb"
        );
    }
    // Every verb the CLI surface documents is now built, so there is no
    // exclusion list left to assert. The positive loop above is the whole check.
}

/// Acceptance: both installed artifacts carry the installed binary name.
///
/// A completion script registers against a command name and a man page is
/// filed under one, so these two land in shared directories where the shell
/// predecessor's own artifacts already sit. Naming the binary is what keeps
/// them from colliding, and what stops the page describing a command the user
/// cannot type
/// ([ADR-0092](../docs/decisions/ADR-0092-namespace-apart-from-the-predecessor.md)).
#[test]
fn generated_artifacts_name_the_installed_binary() {
    let harness = Harness::new();
    assert!(
        man_page(&harness).contains(".TH claude-session-rs 1"),
        "the page is filed under another command"
    );
    for (shell, _) in SHELLS {
        let script = completion(&harness, shell);
        assert!(
            !script.contains("claude-session ") && !script.contains("'claude-session'"),
            "the {shell} script registers the predecessor's spelling"
        );
    }
}

/// Acceptance: the page is derived from the same parser tree.
///
/// The authored phrase is the proof that matters. It lives in `src/ui/help.txt`
/// and reaches the parser's long help and this page through one `include_str!`,
/// so finding it here is finding the shared source rather than a coincidence
/// ([ADR-0016](../docs/decisions/ADR-0016-ship-man-pages.md)).
#[test]
fn man_derives_the_root_page_from_the_parser_tree() {
    let harness = Harness::new();
    let page = man_page(&harness);
    assert!(page.contains(".TH claude-session-rs 1"), "no roff title");
    assert!(page.contains(".SH NAME"), "no NAME section");

    let readable = unescape_roff(&page);
    assert!(
        readable.contains("Unknown arguments are forwarded unchanged"),
        "the authored prose never reached the page"
    );
    assert!(
        readable.contains(env!("CARGO_PKG_VERSION")),
        "the page states no version"
    );

    // Roff is not data, so the verb declares no `--json` and the parser rejects
    // it rather than the wrapper quietly ignoring a formatting request.
    let rejected = Harness::new();
    let output = rejected
        .assert_command()
        .args(["man", "--json"])
        .output()
        .expect("man --json");
    assert_eq!(output.status.code(), Some(64));
    assert!(output.stdout.is_empty());
}

/// Acceptance: generated artifacts leave child argv opaque.
///
/// Driven from the checked-in inventory rather than a hand-picked list, so
/// refreshing the child measurement automatically widens what this asserts.
#[test]
fn generated_artifacts_leave_child_argv_opaque() {
    #[derive(serde::Deserialize)]
    struct Inventory {
        top_level_flags: Vec<String>,
    }

    let fixture = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/child-inventory.yaml"
    ))
    .expect("child inventory");
    let inventory: Inventory = serde_yaml_ng::from_str(&fixture).expect("inventory parses");

    // The wrapper's own denylist, which the artifacts must carry precisely
    // because the wrapper claims those spellings, plus its verb-level flags.
    let claimed = [
        "--verbose",
        "--quiet",
        "--config",
        "--account",
        "--profile",
        "--version",
        "--help",
        "--json",
        "--list",
        "--strict",
    ];
    let child: Vec<&String> = inventory
        .top_level_flags
        .iter()
        .filter(|flag| flag.starts_with("--"))
        .filter(|flag| !claimed.contains(&flag.as_str()))
        .collect();
    assert!(
        child.len() > 40,
        "the inventory yielded only {} child flags, so this gate proves little",
        child.len()
    );

    let harness = Harness::new();
    let script = completion(&harness, "bash");
    let page = unescape_roff(&man_page(&harness));

    // A gate that matches nothing reports success, so prove the haystacks
    // actually carry the spellings the wrapper does claim before asserting the
    // child's are missing.
    assert!(script.contains("--verbose"), "the script proves nothing");
    assert!(page.contains("--verbose"), "the page proves nothing");

    for flag in child {
        assert!(
            !script.contains(flag.as_str()),
            "the bash completion script leaks the child flag {flag}"
        );
        assert!(
            !page.contains(flag.as_str()),
            "the man page leaks the child flag {flag}"
        );
    }
}
