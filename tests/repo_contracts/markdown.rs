//! The Markdown primitives the gates share.
//!
//! This is not a Markdown parser and must not become one. It recognizes only
//! the dialect the formatter emits, and every function fails closed on
//! anything else.

/// Every `## ` heading line, in order, ignoring fenced blocks.
///
/// The shell predecessor used a bare `grep '^## '`, which would read a heading
/// inside a fenced example as a real one. Nothing in the corpus does that
/// today; skipping fences closes the hole before something does.
pub(crate) fn headings(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut fence: Option<(char, usize)> = None;
    for line in text.lines() {
        if let Some(open) = fence {
            if closes_fence(line, open) {
                fence = None;
            }
            continue;
        }
        if let Some(open) = opens_fence(line) {
            fence = Some(open);
            continue;
        }
        if line.starts_with("## ") {
            out.push(line.to_string());
        }
    }
    out
}

/// The lines strictly between `start` and `end`, as `(1-based line, text)`.
///
/// `end` is matched but not emitted; when it never appears the section runs to
/// the end of the file, exactly as the awk ranges it replaces did.
pub(crate) fn section<'a>(text: &'a str, start: &str, end: &str) -> Vec<(usize, &'a str)> {
    let mut out = Vec::new();
    let mut active = false;
    for (index, line) in text.lines().enumerate() {
        if line == start {
            active = true;
            continue;
        }
        if line == end {
            active = false;
        }
        if active {
            out.push((index + 1, line));
        }
    }
    out
}

/// Every line after `heading`, as `(1-based line, text)`.
pub(crate) fn after<'a>(text: &'a str, heading: &str) -> Vec<(usize, &'a str)> {
    let mut out = Vec::new();
    let mut active = false;
    for (index, line) in text.lines().enumerate() {
        if active {
            out.push((index + 1, line));
        }
        if line == heading {
            active = true;
        }
    }
    out
}

/// The first line of a section that is not blank. Blankness is emptiness after
/// trimming, matching awk's `NF` test on a whitespace-only line.
pub(crate) fn first_nonblank<'a>(lines: &[(usize, &'a str)]) -> Option<&'a str> {
    lines
        .iter()
        .find(|(_, line)| !line.trim().is_empty())
        .map(|(_, line)| *line)
}

/// Every inline-link destination on the line: the text between `](` and the
/// next `)`.
pub(crate) fn link_targets(line: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = line;
    while let Some(opener) = rest.find("](") {
        rest = &rest[opener + 2..];
        let Some(closer) = rest.find(')') else { break };
        out.push(&rest[..closer]);
        rest = &rest[closer + 1..];
    }
    out
}

/// True when the text contains an `ADR-NNNN` reference.
pub(crate) fn names_a_record(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes
        .windows(8)
        .any(|window| window.starts_with(b"ADR-") && window[4..].iter().all(u8::is_ascii_digit))
}

/// The opening fence on this line, as `(character, length)`.
///
/// Up to three leading spaces, then three or more backticks or tildes. An info
/// string may follow, which is why the match is not anchored at the end.
pub(crate) fn opens_fence(line: &str) -> Option<(char, usize)> {
    let indent = line.len() - line.trim_start().len();
    if indent > 3 || !line[..indent].chars().all(char::is_whitespace) {
        return None;
    }
    let rest = line.trim_start();
    let marker = rest.chars().next()?;
    if marker != '`' && marker != '~' {
        return None;
    }
    let run = rest.chars().take_while(|c| *c == marker).count();
    (run >= 3).then_some((marker, run))
}

/// A fence closes on a run of the same character, at least as long as the
/// opener, with nothing after it but whitespace.
pub(crate) fn closes_fence(line: &str, open: (char, usize)) -> bool {
    let (marker, length) = open;
    let indent = line.len() - line.trim_start().len();
    if indent > 3 {
        return false;
    }
    let rest = line.trim_start();
    let run = rest.chars().take_while(|c| *c == marker).count();
    run >= length && rest[run..].trim().is_empty()
}

/// Removes inline-code spans, where a run of backticks is closed by a run at
/// least as long.
///
/// This reproduces `s/(`+)[^`]*\1//g`. A backreference is why no regular
/// expression crate could have replaced it: the closing run's length is not
/// known until the opening run is read. The opener is greedy and backtracks,
/// which is what makes ```` ```` ```` strip as two pairs rather than fail; on
/// total failure the scan emits one character and retries from the next, which
/// is the leftmost-match retry the substitution performs.
pub(crate) fn strip_code_spans(line: &str) -> String {
    let chars: Vec<char> = line.chars().collect();
    let mut out = String::with_capacity(line.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '`' {
            out.push(chars[i]);
            i += 1;
            continue;
        }
        let open = run_length(&chars, i);
        let mut matched = None;
        for length in (1..=open).rev() {
            let body = i + length;
            let Some(close) = (body..chars.len()).find(|index| chars[*index] == '`') else {
                continue;
            };
            if run_length(&chars, close) >= length {
                matched = Some(close + length);
                break;
            }
        }
        match matched {
            Some(end) => i = end,
            None => {
                out.push(chars[i]);
                i += 1;
            }
        }
    }
    out
}

/// Removes inline-link destinations, leaving the bracketed text. A destination
/// is syntax rather than prose, so its underscores are not emphasis.
pub(crate) fn strip_link_targets(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(opener) = rest.find("](") {
        out.push_str(&rest[..opener + 1]);
        rest = &rest[opener + 2..];
        match rest.find(')') {
            Some(closer) => rest = &rest[closer + 1..],
            None => {
                rest = "";
                break;
            }
        }
    }
    out.push_str(rest);
    out
}

fn run_length(chars: &[char], from: usize) -> usize {
    chars[from..].iter().take_while(|c| **c == '`').count()
}

// The primitives above are shared by four gates, so each carries its own table
// rather than being covered only through whichever gate happens to exercise it.

#[test]
fn backtick_runs_strip_by_matching_length() {
    assert_eq!(strip_code_spans("a `code` b"), "a  b");
    // Verified against `sed -E 's/(`+)[^`]*\1//g'`: the greedy opener fails at
    // length two, backtracks to one, and closes on the very next backtick, so
    // the leftover run is literal text rather than a span.
    assert_eq!(strip_code_spans("a ``co`de`` b"), "a co` b");
    assert_eq!(
        strip_code_spans("The ``a `x` c`` spelling"),
        "The a  c spelling"
    );
    assert_eq!(strip_code_spans("`a` and `b`"), " and ");
    // A four-run opens and closes itself as two pairs, exactly as the
    // substitution's greedy opener backtracks to length two.
    assert_eq!(strip_code_spans("x````y"), "xy");
    // An unmatched run is literal text, not the start of a span.
    assert_eq!(strip_code_spans("a ` b"), "a ` b");
    assert_eq!(strip_code_spans("no ticks"), "no ticks");
}

#[test]
fn link_destinations_are_syntax_rather_than_prose() {
    assert_eq!(strip_link_targets("see [a](./a_b.md) now"), "see [a] now");
    assert_eq!(strip_link_targets("[x](y) [z](w)"), "[x] [z]");
    assert_eq!(strip_link_targets("no links"), "no links");
}

#[test]
fn a_heading_inside_a_fence_is_not_a_heading() {
    let text = "## Real\n\n```markdown\n## Fake\n```\n\n## Also real\n";
    assert_eq!(headings(text), vec!["## Real", "## Also real"]);
}

#[test]
fn a_longer_fence_does_not_close_on_a_shorter_run() {
    let text = "````\n```\n## Hidden\n````\n\n## Seen\n";
    assert_eq!(headings(text), vec!["## Seen"]);
}

#[test]
fn a_section_ends_at_its_next_heading() {
    let text = "## A\nfirst\n\n## B\nsecond\n";
    assert_eq!(section(text, "## A", "## B"), vec![(2, "first"), (3, "")]);
}

#[test]
fn an_unterminated_section_runs_to_the_end() {
    assert_eq!(section("## A\nfirst\nsecond\n", "## A", "## B").len(), 2);
}

#[test]
fn record_references_are_recognized_by_shape() {
    assert!(names_a_record("./ADR-0074-title.md"));
    assert!(!names_a_record("../../tests/repo_contracts/boundaries.rs"));
    assert!(!names_a_record("ADR-74"));
}
