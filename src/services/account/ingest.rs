//! Reading one wrapper-bound credential from the two sources the rule permits.
//!
//! Shared by the two logins that bring their own secret, because
//! [ADR-0027](../../../docs/decisions/ADR-0027-ingest-secrets-only-from-stdin-or-a-terminal.md)
//! governs the entry of a credential rather than of one particular credential:
//! standard input, or the controlling terminal with echo disabled, and nothing
//! else. What differs between the two callers is the word in the prompt and in
//! the refusal, which is the whole of [`Subject`].

use std::io::Read as _;

use crate::{
    adapters::terminal::Terminal,
    context::AppContext,
    domain::secret::{Secret, SecretError},
    error::{AppError, Diagnostic, ErrorKind},
};

/// Which credential an ingest is reading, for the wording alone.
///
/// Not a behaviour switch: both are read the same two ways and bounded the same
/// way. A person told "the token was not accepted" while pasting a refresh
/// token would reasonably wonder which of the two the wrapper meant.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Subject {
    Token,
    RefreshToken,
}

impl Subject {
    const fn noun(self) -> &'static str {
        match self {
            Self::Token => "token",
            Self::RefreshToken => "refresh token",
        }
    }

    /// The escape a prompting ingest names when it has no terminal.
    fn stdin_escape(self) -> String {
        match self {
            Self::Token => {
                "run account login --token --stdin with the token on standard input".to_owned()
            }
            Self::RefreshToken => concat!(
                "run account login --refresh-token --stdin with the refresh token on ",
                "standard input"
            )
            .to_owned(),
        }
    }
}

/// Refuses when there is no controlling terminal to prompt on.
pub(crate) fn require_terminal(context: &AppContext, subject: Subject) -> Result<(), AppError> {
    let unavailable = |why: String| {
        AppError::new(
            ErrorKind::Unavailable,
            Diagnostic::new(
                format!("{} entry needs a controlling terminal", subject.noun()),
                "/dev/tty",
                why,
                subject.stdin_escape(),
            ),
        )
    };
    match context.adapters().terminal().available() {
        Ok(true) => Ok(()),
        Ok(false) => Err(unavailable(
            "no controlling terminal is available".to_owned(),
        )),
        Err(error) => Err(unavailable(error.to_string())),
    }
}

/// Reads one line from the controlling terminal with echo disabled.
pub(crate) fn read_terminal(
    context: &AppContext,
    subject: Subject,
    prompt: &str,
) -> Result<Secret, AppError> {
    context
        .adapters()
        .terminal()
        .read_secret(prompt)
        .map_err(|error| {
            AppError::new(
                ErrorKind::Unavailable,
                Diagnostic::new(
                    format!("the {} could not be read from the terminal", subject.noun()),
                    "/dev/tty",
                    error.to_string(),
                    subject.stdin_escape(),
                ),
            )
        })?
        .map_err(|reason| refused(subject, reason, &format!("the pasted {}", subject.noun())))
}

/// Reads standard input to end of file and requires exactly one line.
///
/// To end of file rather than one line: a reader that took the first line and
/// discarded the rest would silently accept a two-line paste and store half of
/// what the user meant. Bounded by [`Secret::INGEST_BOUND`] rather than by the
/// stream's own end, so the refusal costs one byte past the limit instead of
/// however much a pipe chooses to send.
pub(crate) fn read_stdin(subject: Subject) -> Result<Secret, AppError> {
    let bytes = read_bounded(std::io::stdin().lock()).map_err(|error| {
        AppError::new(
            ErrorKind::NoInput,
            Diagnostic::new(
                format!(
                    "the {} could not be read from standard input",
                    subject.noun()
                ),
                "standard input",
                error.to_string(),
                format!(
                    "supply the {} on standard input, as one line",
                    subject.noun()
                ),
            ),
        )
    })?;
    Secret::parse_line(&bytes).map_err(|reason| refused(subject, reason, "standard input"))
}

/// Reads at most [`Secret::INGEST_BOUND`] bytes from one ingest stream.
///
/// Taking one byte past the limit is what keeps the refusal and the bound in
/// the same place: the caller's `parse_line` still decides, and it decides from
/// a buffer that can never exceed a credential's accepted length by more than
/// the single byte that proves it was exceeded.
fn read_bounded(reader: impl std::io::Read) -> std::io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader.take(Secret::INGEST_BOUND).read_to_end(&mut bytes)?;
    Ok(bytes)
}

/// Turns a refused line into the login's own usage error.
///
/// The reason never quotes the input, so the subject is substituted into the
/// wording rather than the value.
fn refused(subject: Subject, reason: SecretError, where_: &str) -> AppError {
    AppError::new(
        ErrorKind::Usage,
        Diagnostic::new(
            format!("the {} was not accepted", subject.noun()),
            where_.to_owned(),
            reason.reason().to_owned(),
            format!(
                "supply exactly one line holding the {} and nothing else",
                subject.noun()
            ),
        ),
    )
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    /// The bound, as a length, for the two tests that build inputs from it.
    fn bound() -> usize {
        usize::try_from(Secret::INGEST_BOUND).expect("the ingest bound fits a usize")
    }

    /// The bound has to live on the read rather than on the parse, or an ingest
    /// source that never reaches end of file would be held in memory in full
    /// before the length check could refuse it. `io::repeat` is exactly that
    /// source: this test does not terminate at all if the read is unbounded.
    #[test]
    fn an_endless_ingest_stream_is_refused_without_being_consumed() {
        let bytes = read_bounded(std::io::repeat(b'a')).expect("a bounded read succeeds");
        assert_eq!(bytes.len(), bound());
        assert_eq!(
            Secret::parse_line(&bytes).err(),
            Some(SecretError::TooLong),
            "one byte past the limit is what proves the limit was exceeded"
        );
    }

    /// The bound is one byte past the accepted length, so a credential exactly
    /// at the limit still survives the same reader.
    #[test]
    fn a_credential_at_the_limit_survives_the_bounded_read() {
        let source = vec![b'a'; bound() - 1];
        let bytes = read_bounded(source.as_slice()).expect("a bounded read succeeds");
        assert_eq!(bytes, source);
        assert!(Secret::parse_line(&bytes).is_ok());
    }

    /// Both callers share every mechanism here, so the only thing that can
    /// differ between them is the word a person reads.
    #[test]
    fn each_subject_names_itself_in_its_own_refusal() {
        let token = refused(Subject::Token, SecretError::Empty, "standard input");
        let refresh = refused(Subject::RefreshToken, SecretError::Empty, "standard input");
        assert_eq!(token.diagnostic().what, "the token was not accepted");
        assert_eq!(
            refresh.diagnostic().what,
            "the refresh token was not accepted"
        );
        assert!(refresh.diagnostic().hint.contains("refresh token"));
    }
}
