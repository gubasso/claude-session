//! Raw terminal writer, diagnostics, delimiters, JSON, and color policy.

use std::io::{self, IsTerminal, Write};

use crate::error::AppError;

/// Stateless access to process terminal streams.
pub(crate) struct OutputWriter;

/// The resolved color decision for one invocation, one field per destination.
///
/// Resolving it once is the contract [the presentation reference] states: a
/// renderer reads the field for the stream it writes rather than re-deriving
/// the ladder, so two surfaces cannot disagree about one invocation. The two
/// streams resolve separately because the ladder's last rung asks whether the
/// destination is a terminal, and a redirected standard output says nothing
/// about standard error.
///
/// [the presentation reference]: ../../docs/reference/presentation.md#colour
#[derive(Clone, Copy, Debug)]
pub(crate) struct Color {
    stdout: bool,
    stderr: bool,
}

impl Color {
    /// Resolves both destinations, with machine mode dominating the ladder.
    pub(crate) fn resolve(
        environment: &[(std::ffi::OsString, std::ffi::OsString)],
        mode: crate::commands::dispatch::OutputMode,
        stdout_is_terminal: bool,
        stderr_is_terminal: bool,
    ) -> Self {
        // No JSON document and no log record carries an escape byte, whatever
        // the environment asked for, so machine mode short-circuits the ladder
        // rather than being a rung inside it.
        if matches!(mode, crate::commands::dispatch::OutputMode::Json) {
            return Self {
                stdout: false,
                stderr: false,
            };
        }
        Self {
            stdout: OutputWriter::color(environment, stdout_is_terminal),
            stderr: OutputWriter::color(environment, stderr_is_terminal),
        }
    }
    /// Reports whether standard output carries color.
    pub(crate) const fn stdout(self) -> bool {
        self.stdout
    }
    /// Reports whether standard error carries color.
    #[allow(
        dead_code,
        reason = "the diagnostic renderer applies it in its own slice"
    )]
    pub(crate) const fn stderr(self) -> bool {
        self.stderr
    }
}

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
    /// Resolves the first-match color ladder owned by [the presentation
    /// reference].
    ///
    /// Rungs 1 and 2 test presence and non-emptiness and never read the value:
    /// reading it would invent a third convention for a question two published
    /// ones already answer ([ADR-0083]). Force precedes deny because the
    /// spelling is a request to force, not a boolean.
    ///
    /// [the presentation reference]: ../../docs/reference/presentation.md#colour
    /// [ADR-0083]: ../../docs/decisions/ADR-0083-read-only-the-two-published-colour-variables.md
    pub(crate) fn color(
        environment: &[(std::ffi::OsString, std::ffi::OsString)],
        target_is_terminal: bool,
    ) -> bool {
        let active = |name: &str| {
            environment
                .iter()
                .any(|(key, value)| key == name && !value.is_empty())
        };
        // Rung 1 is the only rung that turns colour on against the others.
        if active("FORCE_COLOR") {
            return true;
        }
        // Rungs 2, 3, and 4 all fail closed, so they conjoin rather than
        // cascade. Their order is unobservable once rung 1 has not matched.
        let dumb = environment
            .iter()
            .any(|(key, value)| key == "TERM" && value == "dumb");
        !active("NO_COLOR") && !dumb && target_is_terminal
    }
    /// Reports whether stdout is a terminal through the output boundary.
    pub(crate) fn stdout_is_terminal(&self) -> bool {
        io::stdout().is_terminal()
    }
    /// Reports whether stderr is a terminal through the output boundary.
    ///
    /// Diagnostics and the log mirror land here, so the ladder's terminal rung
    /// needs this stream's own answer rather than standard output's.
    pub(crate) fn stderr_is_terminal(&self) -> bool {
        io::stderr().is_terminal()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Rung 1 precedes rung 2, so the two overrides together are not a
    /// contradiction the resolver has to guess at.
    #[test]
    fn forced_color_wins_over_denied_color() {
        assert!(OutputWriter::color(
            &[
                ("NO_COLOR".into(), "1".into()),
                ("FORCE_COLOR".into(), "1".into())
            ],
            false
        ));
        assert!(!OutputWriter::color(
            &[("NO_COLOR".into(), "0".into())],
            true
        ));
        assert!(OutputWriter::color(
            &[("FORCE_COLOR".into(), "0".into())],
            false
        ));
    }

    /// Both conventions declare an empty value inert, which is the half of each
    /// test a presence-only reading silently fails.
    #[test]
    fn an_empty_override_is_inert() {
        assert!(OutputWriter::color(&[("NO_COLOR".into(), "".into())], true));
        assert!(!OutputWriter::color(
            &[("FORCE_COLOR".into(), "".into())],
            false
        ));
        assert!(!OutputWriter::color(
            &[
                ("FORCE_COLOR".into(), "".into()),
                ("TERM".into(), "dumb".into())
            ],
            true
        ));
    }

    /// The two lower rungs fail closed, and neither outranks an active
    /// override.
    #[test]
    fn a_dumb_or_redirected_destination_fails_closed() {
        assert!(!OutputWriter::color(
            &[("TERM".into(), "dumb".into())],
            true
        ));
        assert!(!OutputWriter::color(&[], false));
        assert!(OutputWriter::color(&[], true));
        assert!(OutputWriter::color(
            &[
                ("TERM".into(), "dumb".into()),
                ("FORCE_COLOR".into(), "1".into())
            ],
            false
        ));
    }

    /// Machine output is undecorated whatever the environment asked for, so
    /// the strongest override the ladder has still loses to JSON mode.
    #[test]
    fn machine_mode_dominates_every_override() {
        let forced = [("FORCE_COLOR".into(), "1".into())];
        let decision = Color::resolve(
            &forced,
            crate::commands::dispatch::OutputMode::Json,
            true,
            true,
        );
        assert!(!decision.stdout());
        assert!(!decision.stderr());
    }

    /// One redirected stream must not decide the other's appearance: a piped
    /// standard output is the common case, and the diagnostic still lands on a
    /// terminal.
    #[test]
    fn each_destination_resolves_its_own_terminal_rung() {
        let decision = Color::resolve(
            &[],
            crate::commands::dispatch::OutputMode::Human,
            false,
            true,
        );
        assert!(!decision.stdout());
        assert!(decision.stderr());
        // An active override still covers both, since rung 1 never consults
        // the destination.
        let decision = Color::resolve(
            &[("FORCE_COLOR".into(), "1".into())],
            crate::commands::dispatch::OutputMode::Human,
            false,
            false,
        );
        assert!(decision.stdout());
        assert!(decision.stderr());
    }
}
