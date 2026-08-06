//! Decorative emphasis is forbidden by AGENTS.md. A source-aware scan avoids
//! treating examples, identifiers, link destinations, or Markdown punctuation
//! as prose emphasis.
//!
//! The shell predecessor received the changed files from the hook. A test has
//! no such list, so it enumerates the tree itself. That scans more files per
//! run and is the one behaviour this port deliberately changes.

use crate::markdown::{closes_fence, opens_fence, strip_code_spans, strip_link_targets};
use crate::violation::{Violation, assert_clean};
use crate::{read, tree};

const SUFFIXES: &[&str] = &[".md", ".markdown"];

/// A floor and sentinels, so a walk that quietly stops finding files fails
/// rather than reporting a clean tree. There are 119 documents today.
const FLOOR: usize = 110;
const SENTINELS: &[&str] = &[
    "AGENTS.md",
    "README.md",
    "docs/decisions/template.md",
    "docs/plan/milestones.md",
    "docs/reference/testing-and-quality.md",
];

fn documents() -> Vec<String> {
    tree::files("", SUFFIXES)
}

/// The reason inside an `allow-emphasis` comment, when the line is one.
///
/// `Err` marks the empty-reason form, which is its own violation: an exemption
/// with no stated reason is the thing the marker exists to prevent.
fn allowance(line: &str) -> Option<Result<(), ()>> {
    let trimmed = line.trim();
    let inner = trimmed.strip_prefix("<!--")?.strip_suffix("-->")?;
    let rest = inner.trim_start().strip_prefix("allow-emphasis:")?;
    let reason = rest.trim_start();
    if reason.is_empty() {
        return Some(Err(()));
    }
    // The reason may not contain `<`, so a nested comment cannot smuggle one
    // exemption past the end of another.
    if reason.contains('<') {
        return None;
    }
    Some(Ok(()))
}

/// Three or more of `*`, `_`, or `-` with nothing else but space is a
/// horizontal rule, not emphasis.
fn is_rule(line: &str) -> bool {
    let marks = line
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<Vec<_>>();
    marks.len() >= 3 && marks.iter().all(|c| matches!(c, '*' | '_' | '-'))
}

/// Drops a leading list marker and the whitespace after it, so a starred
/// bullet is not read as italics.
fn without_list_marker(line: &str) -> String {
    let indent = line.len() - line.trim_start().len();
    let rest = line.trim_start();
    let Some(marker) = rest.chars().next() else {
        return line.to_string();
    };
    if !matches!(marker, '*' | '+' | '-') {
        return line.to_string();
    }
    let after = &rest[marker.len_utf8()..];
    let body = after.trim_start();
    if after.len() == body.len() {
        return line.to_string();
    }
    format!("{}{body}", &line[..indent])
}

fn paired(chars: &[char], mark: char) -> Option<(usize, usize)> {
    let length = chars.len();
    for open in 0..length.saturating_sub(1) {
        if chars[open] != mark || chars[open + 1] != mark {
            continue;
        }
        let body = open + 2;
        if body >= length || chars[body] == mark || chars[body].is_whitespace() {
            continue;
        }
        let mut close = body + 1;
        while close < length && chars[close] != mark {
            close += 1;
        }
        if close + 1 < length && chars[close] == mark && chars[close + 1] == mark {
            return Some((open, close + 2));
        }
    }
    None
}

fn single(chars: &[char], mark: char, boundary: fn(char) -> bool) -> Option<(usize, usize)> {
    let length = chars.len();
    for open in 0..length {
        if chars[open] != mark {
            continue;
        }
        if open > 0 && boundary(chars[open - 1]) {
            continue;
        }
        let body = open + 1;
        if body >= length || chars[body] == mark || chars[body].is_whitespace() {
            continue;
        }
        let mut close = body + 1;
        while close < length && chars[close] != mark {
            close += 1;
        }
        if close >= length {
            continue;
        }
        if close + 1 < length && boundary(chars[close + 1]) {
            continue;
        }
        let start = open.saturating_sub(1);
        let end = if close + 1 < length {
            close + 2
        } else {
            close + 1
        };
        return Some((start, end));
    }
    None
}

fn is_star(c: char) -> bool {
    c == '*'
}

fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// The first emphasis span on an already-cleaned line, in the order the four
/// forms are tried: strong before plain, so `**a**` is not reported as `*a*`.
fn emphasis(clean: &str) -> Option<String> {
    let chars: Vec<char> = clean.chars().collect();
    let found = paired(&chars, '*')
        .or_else(|| paired(&chars, '_'))
        .or_else(|| single(&chars, '*', is_star))
        .or_else(|| single(&chars, '_', is_word))?;
    Some(chars[found.0..found.1].iter().collect())
}

pub(crate) fn scan(path: &str, text: &str) -> Vec<Violation> {
    let mut found = Vec::new();
    let mut fence: Option<(char, usize)> = None;
    let mut allow_next = false;

    for (index, line) in text.lines().enumerate() {
        let number = index + 1;

        if let Some(open) = fence {
            if closes_fence(line, open) {
                fence = None;
            }
            continue;
        }
        if let Some(open) = opens_fence(line) {
            fence = Some(open);
            allow_next = false;
            continue;
        }

        match allowance(line) {
            Some(Err(())) => {
                found.push(Violation::at(path, number, "empty allow-emphasis reason"));
                allow_next = false;
                continue;
            }
            Some(Ok(())) => {
                allow_next = true;
                continue;
            }
            None => {}
        }

        if allow_next {
            // The formatter inserts a blank line after a standalone HTML
            // comment, so the exemption has to survive the gap it creates or it
            // can never be used.
            if line.trim().is_empty() {
                continue;
            }
            allow_next = false;
            continue;
        }

        // Emphasis requires one of these two delimiters, so lines without
        // either skip the stripping below. This keeps the whole-tree gate
        // cheap enough for the push budget.
        if !line.contains('*') && !line.contains('_') {
            continue;
        }

        // Equal-length backtick runs delimit inline code. Link destinations are
        // syntax rather than prose, and escaped delimiters are literal.
        let clean = strip_link_targets(&strip_code_spans(line))
            .replace("\\*", "")
            .replace("\\_", "");

        if is_rule(&clean) {
            continue;
        }
        if let Some(span) = emphasis(&without_list_marker(&clean)) {
            found.push(Violation::at(path, number, span));
        }
    }

    found
}

#[test]
fn every_markdown_file_is_free_of_decorative_emphasis() {
    let mut found = Vec::new();
    for path in documents() {
        found.extend(scan(&path, &read(&path)));
    }
    assert_clean(&found);
}

#[test]
fn the_walk_reaches_the_whole_repository() {
    let documents = documents();
    assert!(
        documents.len() >= FLOOR,
        "found {} documents, expected at least {FLOOR}",
        documents.len()
    );
    for sentinel in SENTINELS {
        assert!(
            documents.iter().any(|path| path == sentinel),
            "{sentinel} was not recovered by the walk"
        );
    }
}

/// Build output is not repository prose. `cargo package` writes a second copy
/// of the root README under `target`, so without the exclusion every finding
/// there would be reported twice and the gate's output would depend on whether
/// anyone had packaged the crate.
#[test]
fn the_walk_excludes_build_output() {
    for path in documents() {
        assert!(
            !path.starts_with("target/") && !path.contains("/.git/") && !path.contains("/.draft/"),
            "{path} is build or draft state and must not be gated"
        );
    }
}

// The tests below prove the gate can go red, driven from literals.

fn spans(text: &str) -> Vec<String> {
    scan("doc.md", text)
        .iter()
        .map(ToString::to_string)
        .collect()
}

fn flags(text: &str) -> bool {
    !scan("doc.md", text).is_empty()
}

#[test]
fn bold_prose_is_a_violation() {
    assert_eq!(spans("This is **bold** prose.\n"), ["doc.md:1: **bold**"]);
}

#[test]
fn underscore_bold_is_a_violation() {
    assert_eq!(spans("This is __bold__ prose.\n"), ["doc.md:1: __bold__"]);
}

#[test]
fn single_star_italics_is_a_violation() {
    assert!(flags("This is *italic* prose.\n"));
}

#[test]
fn single_underscore_italics_is_a_violation() {
    assert!(flags("This is _italic_ prose.\n"));
}

#[test]
fn emphasis_inside_a_fence_is_ignored() {
    assert!(!flags("```rust\nlet x = **y**;\n```\n"));
}

#[test]
fn emphasis_inside_a_tilde_fence_is_ignored() {
    assert!(!flags("~~~\n**bold**\n~~~\n"));
}

#[test]
fn a_longer_fence_is_not_closed_by_a_shorter_run() {
    assert!(!flags("````\n```\n**bold**\n````\n"));
}

#[test]
fn emphasis_inside_a_backtick_run_is_ignored() {
    assert!(!flags("The `**literal**` spelling.\n"));
    assert!(!flags("The ``a `**b**` c`` spelling.\n"));
}

#[test]
fn a_link_destination_with_underscores_is_ignored() {
    assert!(!flags("See [the page](./some_long_name.md) for more.\n"));
}

#[test]
fn an_escaped_delimiter_is_ignored() {
    assert!(!flags("A literal \\*star\\* here.\n"));
}

#[test]
fn a_horizontal_rule_is_ignored() {
    assert!(!flags("***\n"));
    assert!(!flags("* * *\n"));
    assert!(!flags("___\n"));
}

#[test]
fn a_star_list_marker_is_not_italics() {
    assert!(!flags("* one item\n"));
}

#[test]
fn a_snake_case_identifier_in_prose_is_not_italics() {
    assert!(!flags("The value some_long_name holds it.\n"));
}

#[test]
fn an_allow_emphasis_comment_exempts_the_next_prose_line() {
    assert!(!flags(
        "<!-- allow-emphasis: quoting upstream -->\nThis is **bold**.\n"
    ));
}

#[test]
fn an_allow_emphasis_comment_survives_the_formatters_blank_line() {
    assert!(!flags(
        "<!-- allow-emphasis: quoting upstream -->\n\nThis is **bold**.\n"
    ));
}

#[test]
fn an_allow_emphasis_comment_exempts_only_one_line() {
    assert!(flags(
        "<!-- allow-emphasis: quoting upstream -->\nFirst **bold**.\nSecond **bold**.\n"
    ));
}

#[test]
fn an_empty_allow_emphasis_reason_is_a_violation() {
    let found = spans("<!-- allow-emphasis: -->\nPlain prose.\n");
    assert_eq!(found, ["doc.md:1: empty allow-emphasis reason"]);
}
