//! The finding type every gate returns, and its rendering.
//!
//! `file` is a `String` where the collision audit uses a `&'static str`,
//! because the emphasis gate discovers its paths by walking the tree at run
//! time and so cannot name them as constants.

use std::fmt::{self, Display, Formatter};

#[derive(Debug)]
pub(crate) struct Violation {
    file: String,
    line: Option<usize>,
    message: String,
}

impl Violation {
    pub(crate) fn at(file: impl Into<String>, line: usize, message: impl Into<String>) -> Self {
        Self {
            file: file.into(),
            line: Some(line),
            message: message.into(),
        }
    }

    /// A fact about the whole file, reported without a line, because a
    /// cross-file finding has no single line to blame.
    pub(crate) fn whole(file: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            file: file.into(),
            line: None,
            message: message.into(),
        }
    }
}

impl Display for Violation {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.line {
            Some(line) => write!(f, "{}:{}: {}", self.file, line, self.message),
            None => write!(f, "{}: {}", self.file, self.message),
        }
    }
}

/// Sorted so a failure reads the same on every host and every run.
pub(crate) fn render(violations: &[Violation]) -> String {
    let mut lines: Vec<String> = violations.iter().map(ToString::to_string).collect();
    lines.sort();
    let count = violations.len();
    lines.push(format!(
        "{count} violation{}",
        if count == 1 { "" } else { "s" }
    ));
    format!("\n{}", lines.join("\n"))
}

/// Panics with the rendered list when it is not empty. Every gate ends this
/// way, so one failure format serves the whole target.
pub(crate) fn assert_clean(violations: &[Violation]) {
    assert!(violations.is_empty(), "{}", render(violations));
}
