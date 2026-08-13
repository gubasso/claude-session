//! The failure a person reads.
//!
//! The last surface [ADR-0093] named and the last one to meet it. The four
//! fields a diagnostic carries have not changed and the machine document still
//! carries all of them; what changed is that the human side states them as
//! sentences a reader can act on, at the same wrap every other report uses.
//!
//! [ADR-0093]: ../../docs/decisions/ADR-0093-write-every-non-machine-surface-for-a-person.md

use crate::{error::AppError, ui::prose::paragraph};

/// The one wording, decorated or plain.
///
/// The `error[Kind]` token is the diagnostic's coloured surface and carries
/// nothing the kind does not already spell, which is the condition the closed
/// surface set attaches to a colour.
pub(crate) fn render(error: &AppError, color: bool) -> String {
    let diagnostic = error.diagnostic();
    let token = format!("error[{}]", error.kind().spelling());
    let mut out = format!(
        "claude-session-rs: {}: {}\n\n",
        if color {
            format!("\u{1b}[31m{token}\u{1b}[0m")
        } else {
            token
        },
        diagnostic.what
    );
    out.push_str(&body(&diagnostic.why));
    out.push_str(&paragraph(&format!("This concerns {}.", diagnostic.where_)));
    if let Some(code) = diagnostic.child_exit {
        out.push_str(&paragraph(&format!("claude itself exited {code}.")));
    }
    out.push('\n');
    out.push_str(&paragraph(&format!(
        "What to do: {}",
        sentence(&diagnostic.hint)
    )));
    out
}

/// Renders the explanation, reflowing prose but never pre-formatted text.
///
/// A parser's own usage block arrives here with its lines already laid out, and
/// reflowing it would turn a readable grammar into one run-on paragraph. The
/// test is the newline the author put there: prose has none.
fn body(why: &str) -> String {
    if !why.trim_end().contains('\n') {
        return paragraph(&sentence(why));
    }
    let mut out: String = why
        .trim_end()
        .lines()
        .map(|line| {
            if line.trim().is_empty() {
                "\n".to_owned()
            } else {
                format!("  {line}\n")
            }
        })
        .collect();
    // A pre-formatted block ends its own paragraph, so what follows does not
    // read as one more of its lines.
    out.push('\n');
    out
}

/// Finishes a fragment as a sentence, since call sites author both.
fn sentence(text: &str) -> String {
    let trimmed = text.trim_end();
    let mut out = trimmed.to_owned();
    if !trimmed.ends_with('.') && !trimmed.ends_with('!') && !trimmed.ends_with('?') {
        out.push('.');
    }
    if let Some(first) = out.get_mut(0..1) {
        first.make_ascii_uppercase();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{Diagnostic, ErrorKind};

    fn fixture() -> AppError {
        AppError::new(
            ErrorKind::Config,
            Diagnostic::new(
                "child launch is not bound to a complete session",
                "this run",
                "a child launch requires a resolved account and profile",
                "run claude-session-rs account login <name>",
            ),
        )
    }

    /// Rule 1 again, at the one surface whose colour the ladder resolves for
    /// standard error rather than standard output.
    #[test]
    fn colour_changes_no_character_of_the_diagnostic() {
        let coloured = render(&fixture(), true);
        let stripped: String = {
            let mut out = String::new();
            let mut rest = coloured.as_str();
            while let Some(start) = rest.find('\u{1b}') {
                out.push_str(&rest[..start]);
                let after = &rest[start..];
                let end = after.find('m').map_or(after.len(), |index| index + 1);
                rest = &after[end..];
            }
            out.push_str(rest);
            out
        };
        assert_eq!(stripped, render(&fixture(), false));
    }

    /// The labelled `Where:`/`Why:`/`Hint:` triple is what this replaced, so
    /// its absence is the thing worth pinning.
    #[test]
    fn the_diagnostic_carries_no_labelled_field_triple() {
        let text = render(&fixture(), false);
        for label in ["Where:", "Why:", "Hint:", "Child exit:", "Concerning:"] {
            assert!(!text.contains(label), "{label} survives in: {text}");
        }
        assert!(text.contains("What to do:"), "{text}");
    }
}
