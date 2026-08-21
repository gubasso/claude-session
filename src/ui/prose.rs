//! The shared shape of every human report: one wrap, one indent, one palette.
//!
//! Extracted from the `doctor` renderer, which was the first surface written
//! for a person and discovered these while it was the only one. They live here
//! because the rule binds every renderer ([ADR-0093]), and two renderers that
//! wrapped at two columns would make the presentation contract's constant a
//! claim rather than a fact.
//!
//! [ADR-0093]: ../../docs/decisions/ADR-0093-write-every-non-machine-surface-for-a-person.md

use crate::domain::checks::CheckStatus;

/// The column prose starts at: two spaces, the widest status word, two spaces.
pub(crate) const BODY_INDENT: usize = 13;
/// The column every line wraps at. A constant rather than the terminal width,
/// so the bytes into a pipe are the bytes into a terminal.
pub(crate) const WRAP_AT: usize = 76;
/// The indent a report's body sits at under its heading.
pub(crate) const INDENT: &str = "  ";
/// The gap between two table columns.
const GUTTER: &str = "  ";
/// The character the rule under a table's header row is drawn with.
const RULE: char = '\u{2500}';

/// Wraps `text` at [`WRAP_AT`], prefixing the first line and the rest apart.
///
/// A word longer than the remaining room overflows rather than being broken:
/// the long words here are filesystem paths, and a path split across two lines
/// cannot be copied back into a shell.
pub(crate) fn wrap(text: &str, first_prefix: &str, continuation: &str) -> String {
    let mut out = String::new();
    let mut line = first_prefix.to_owned();
    let mut empty = true;
    for word in text.split_whitespace() {
        if !empty && line.chars().count() + 1 + word.chars().count() > WRAP_AT {
            out.push_str(line.trim_end());
            out.push('\n');
            continuation.clone_into(&mut line);
            empty = true;
        }
        if !empty {
            line.push(' ');
        }
        line.push_str(word);
        empty = false;
    }
    if !empty {
        out.push_str(line.trim_end());
        out.push('\n');
    }
    out
}

/// Lays a table out at columns computed from its own rows.
///
/// Returns the header block — the column names and the rule under them — and
/// one laid-out line per row, separately, because a row is coloured after it
/// is laid out and the header carries no colour at all. Every column pads to
/// the widest cell in it, header included, and the last column is not padded,
/// so the same rows produce the same bytes into a pipe and into a terminal
/// ([presentation]). Nothing here reads the terminal, and nothing is cut to
/// fit one: a report that dropped half a name would be answering a question
/// about the window rather than about the sessions.
///
/// A row shorter than the header is padded with empty cells rather than
/// refused, so a caller cannot make the layout disagree with itself.
///
/// [presentation]: ../../docs/reference/presentation.md
pub(crate) fn table(headers: &[&str], rows: &[Vec<String>]) -> (String, Vec<String>) {
    let widths: Vec<usize> = headers
        .iter()
        .enumerate()
        .map(|(column, header)| {
            rows.iter()
                .filter_map(|row| row.get(column))
                .map(|cell| cell.chars().count())
                .chain(std::iter::once(header.chars().count()))
                .max()
                .unwrap_or_default()
        })
        .collect();
    let lay_out = |cells: &[String]| {
        let mut line = INDENT.to_owned();
        for (column, width) in widths.iter().enumerate() {
            if column > 0 {
                line.push_str(GUTTER);
            }
            let cell = cells.get(column).map_or("", String::as_str);
            line.push_str(cell);
            for _ in cell.chars().count()..*width {
                line.push(' ');
            }
        }
        line.trim_end().to_owned()
    };
    let names: Vec<String> = headers.iter().map(|header| (*header).to_owned()).collect();
    let rule: Vec<String> = widths
        .iter()
        .map(|width| RULE.to_string().repeat(*width))
        .collect();
    let header = format!("{}\n{}\n", lay_out(&names), lay_out(&rule));
    (header, rows.iter().map(|row| lay_out(row)).collect())
}

/// Wraps one paragraph at the shared body indent.
pub(crate) fn paragraph(text: &str) -> String {
    wrap(text, INDENT, INDENT)
}

/// The four-bit escapes for the coloured surfaces.
///
/// Nothing here carries meaning: a status word already spells the status and a
/// heading already spells its subject, which is the condition the closed
/// surface set attaches to a colour ([ADR-0082]).
///
/// [ADR-0082]: ../../docs/decisions/ADR-0082-colour-a-closed-set-of-named-surfaces.md
#[derive(Clone, Copy)]
pub(crate) struct Palette(bool);

impl Palette {
    /// Builds a palette that decorates, or one that does nothing.
    pub(crate) const fn new(color: bool) -> Self {
        Self(color)
    }
    pub(crate) fn status(self, status: CheckStatus) -> String {
        let token = format!("[{}]", status.as_str());
        if !self.0 {
            return token;
        }
        let code = match status {
            CheckStatus::Pass => "32",
            CheckStatus::Warn => "33",
            CheckStatus::Fail => "31",
            CheckStatus::Skipped => "2",
        };
        format!("\u{1b}[{code}m{token}\u{1b}[0m")
    }
    /// Colours the one plain status token in already-laid-out text.
    ///
    /// The discipline every renderer follows: lay the row out with the plain
    /// token, wrap it, and colour afterwards. Wrapping on an escaped token
    /// would let colour move a line break, and rule 1 says stripping colour
    /// changes nothing.
    pub(crate) fn recolour(self, text: String, status: CheckStatus) -> String {
        self.recolour_token(text, &format!("[{}]", status.as_str()), status)
    }
    /// Colours a token that spells something other than the status word.
    ///
    /// The verdicts of a session report are statuses in the shape sense — one
    /// bracketed word per row — while spelling their own vocabulary, so they
    /// borrow the colour without borrowing the word ([ADR-0082]).
    ///
    /// [ADR-0082]: ../../docs/decisions/ADR-0082-colour-a-closed-set-of-named-surfaces.md
    pub(crate) fn recolour_token(self, text: String, token: &str, status: CheckStatus) -> String {
        if !self.0 {
            return text;
        }
        let coloured = self.status(status);
        let plain = format!("[{}]", status.as_str());
        text.replacen(token, &coloured.replacen(&plain, token, 1), 1)
    }
    pub(crate) fn heading(self, text: &str) -> String {
        if self.0 {
            format!("\u{1b}[1m{text}\u{1b}[0m")
        } else {
            text.to_owned()
        }
    }
}

/// Picks the singular or plural wording for a count.
pub(crate) const fn plural(count: usize, one: &'static str, many: &'static str) -> &'static str {
    if count == 1 { one } else { many }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prose_wraps_at_a_constant_and_never_splits_a_word() {
        let path = "/very/long/path/".repeat(12);
        let wrapped = wrap(&format!("see {path} for details"), "  ", "    ");
        assert!(wrapped.lines().any(|line| line.contains(&path)));
        for line in wrapped.lines() {
            assert!(
                line.chars().count() <= WRAP_AT || line.contains(&path),
                "{line}"
            );
        }
    }

    /// Rule 1: colour is decoration. Stripping it leaves the same characters,
    /// which is what lets a golden test pin one set of bytes for both.
    #[test]
    fn colour_changes_no_character_of_a_decorated_token() {
        for (plain, decorated) in [
            (
                Palette::new(false).status(CheckStatus::Pass),
                Palette::new(true).status(CheckStatus::Pass),
            ),
            (
                Palette::new(false).heading("Accounts"),
                Palette::new(true).heading("Accounts"),
            ),
        ] {
            assert_eq!(plain, strip(&decorated));
        }
    }

    /// The columns come from the rows, so two callers with the same rows get
    /// the same bytes whatever they are running under.
    #[test]
    fn a_table_pads_every_column_to_its_widest_cell() {
        let (header, rows) = table(
            &["status", "session", "why"],
            &[
                vec![
                    "[live]".to_owned(),
                    "claude-session-53".to_owned(),
                    "running".to_owned(),
                ],
                vec![
                    "[unknown]".to_owned(),
                    "agent-9-1".to_owned(),
                    "no record of what it was".to_owned(),
                ],
            ],
        );
        let lines: Vec<&str> = header.lines().collect();
        assert_eq!(lines[0], "  status     session            why");
        assert_eq!(
            lines[1],
            "  ─────────  ─────────────────  ────────────────────────"
        );
        assert_eq!(rows[0], "  [live]     claude-session-53  running");
        assert_eq!(
            rows[1],
            "  [unknown]  agent-9-1          no record of what it was"
        );
    }

    /// The last column is not padded, so no row carries trailing blanks a
    /// golden test would have to pin and a copy would carry away.
    #[test]
    fn a_table_row_ends_at_its_last_character() {
        let (header, rows) = table(
            &["one", "two"],
            &[vec!["a".to_owned(), "b".to_owned()], vec!["c".to_owned()]],
        );
        for line in header.lines().chain(rows.iter().map(String::as_str)) {
            assert_eq!(line, line.trim_end(), "{line:?} carries trailing blanks");
        }
        assert_eq!(rows[1], "  c");
    }

    /// Removes every SGR sequence, which is the only escape shape emitted.
    fn strip(text: &str) -> String {
        let mut out = String::new();
        let mut rest = text;
        while let Some(start) = rest.find('\u{1b}') {
            out.push_str(&rest[..start]);
            let after = &rest[start..];
            let end = after.find('m').map_or(after.len(), |index| index + 1);
            rest = &after[end..];
        }
        out.push_str(rest);
        out
    }
}
