//! Raw terminal writer, diagnostics, delimiters, JSON, and color policy.

use std::io::{self, IsTerminal, Write};

use crate::error::AppError;

/// Stateless access to process terminal streams.
pub(crate) struct OutputWriter;

/// Renders a failure and returns its code, leaving the log sink alone.
///
/// The log record and the diagnostic are emitted together so they cannot spell
/// the same failure two ways. Flushing is the entry point's to order, not this
/// function's: it only writes.
pub(crate) fn report(error: &AppError, mode: crate::commands::dispatch::OutputMode) -> u8 {
    tracing::error!(
        op = "dispatch",
        status = "err",
        err.kind = error.kind().spelling(),
        "wrapper operation failed"
    );
    let writer = OutputWriter::system();
    // A caller asking for JSON asked for it on both streams.
    match mode {
        crate::commands::dispatch::OutputMode::Human => writer.diagnostic(error),
        crate::commands::dispatch::OutputMode::Json => writer.diagnostic_json(error),
    }
    error.exit_code()
}

/// Wraps a failed terminal write as the typed boundary error.
///
/// Every composed verb needs this and they must agree, since the diagnostic is
/// the same condition whichever stream refused it.
pub(crate) fn output_error(error: &io::Error) -> AppError {
    AppError::new(
        crate::error::ErrorKind::Io,
        crate::error::Diagnostic::new(
            "terminal output failed",
            "standard output",
            error.to_string(),
            "check the output stream",
        ),
    )
}

#[allow(clippy::unused_self)] // The value is the single writer seam carried by AppContext.
impl OutputWriter {
    /// Constructs the system-stream writer.
    pub(crate) const fn system() -> Self {
        Self
    }
    /// Writes exact bytes to standard output.
    pub(crate) fn stdout(&self, bytes: &[u8]) -> io::Result<()> {
        let mut output = io::stdout().lock();
        output.write_all(bytes)?;
        output.flush()
    }
    /// Writes exact bytes to standard error.
    pub(crate) fn stderr(&self, bytes: &[u8]) -> io::Result<()> {
        let mut output = io::stderr().lock();
        output.write_all(bytes)?;
        output.flush()
    }
    /// Renders a wrapper diagnostic once on standard error.
    pub(crate) fn diagnostic(&self, error: &AppError) {
        let _ = self.stderr(format!("{error}\n").as_bytes());
    }
    /// Renders the fixed machine error document on standard error.
    pub(crate) fn diagnostic_json(&self, error: &AppError) {
        let diagnostic = error.diagnostic();
        let mut document = serde_json::json!({
            "kind": error.kind().spelling(),
            "what": diagnostic.what,
            "where": diagnostic.where_,
            "why": diagnostic.why,
            "hint": diagnostic.hint,
        });
        if let Some(child_exit) = diagnostic.child_exit {
            document["child_exit"] = serde_json::json!(child_exit);
        }
        if let Ok(mut bytes) = serde_json::to_vec(&document) {
            bytes.push(b'\n');
            let _ = self.stderr(&bytes);
        }
    }
    /// Writes the exact composed-output delimiter spacing.
    pub(crate) fn delimiter(&self, child_argument: &str) -> io::Result<()> {
        self.stdout(format!("\n\n--- claude {child_argument} ---\n\n").as_bytes())
    }
    /// Resolves the documented first-match color ladder.
    pub(crate) fn color(
        environment: &[(std::ffi::OsString, std::ffi::OsString)],
        target_is_terminal: bool,
    ) -> bool {
        let set = |name: &str| environment.iter().any(|(key, _)| key == name);
        if set("NO_COLOR") {
            false
        } else if set("FORCE_COLOR") {
            true
        } else if environment
            .iter()
            .any(|(key, value)| key == "TERM" && value == "dumb")
        {
            false
        } else {
            target_is_terminal
        }
    }
    /// Reports whether stdout is a terminal through the output boundary.
    pub(crate) fn stdout_is_terminal(&self) -> bool {
        io::stdout().is_terminal()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn color_precedence() {
        assert!(!OutputWriter::color(
            &[
                ("NO_COLOR".into(), "".into()),
                ("FORCE_COLOR".into(), "1".into())
            ],
            true
        ));
        assert!(OutputWriter::color(
            &[
                ("FORCE_COLOR".into(), "".into()),
                ("TERM".into(), "dumb".into())
            ],
            false
        ));
        assert!(!OutputWriter::color(
            &[("TERM".into(), "dumb".into())],
            true
        ));
    }
}
