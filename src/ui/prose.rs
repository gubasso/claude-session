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
        if !self.0 {
            return text;
        }
        let token = format!("[{}]", status.as_str());
        text.replacen(&token, &self.status(status), 1)
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
