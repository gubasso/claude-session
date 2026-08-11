//! The confirmation answer rule.
//!
//! Separated from the terminal adapter because the rule is the interesting part
//! and reading a line is not: everything that is not consent declines, and that
//! is a claim worth a test rather than a claim about `/dev/tty`.

/// What one confirmation answer means.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Consent {
    Granted,
    Declined,
}

impl Consent {
    pub(crate) const fn granted(self) -> bool {
        matches!(self, Self::Granted)
    }
}

/// Decides one answer, asked once.
///
/// `y` and `yes` consent, case-insensitively and ignoring surrounding
/// whitespace. Everything else declines: a bare Enter, an unrecognized word,
/// and end of input. The default is refusal because the question guards a
/// deletion, and because a verb that re-asked would hang against a terminal
/// whose user has already walked away.
pub(crate) fn decide(answer: Option<&str>) -> Consent {
    match answer.map(str::trim) {
        Some(value) if value.eq_ignore_ascii_case("y") || value.eq_ignore_ascii_case("yes") => {
            Consent::Granted
        }
        _ => Consent::Declined,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_y_and_yes_consent() {
        for answer in ["y", "Y", "yes", "YES", " yes \n", "\tY\r\n"] {
            assert_eq!(
                decide(Some(answer)),
                Consent::Granted,
                "{answer:?} consents"
            );
        }
    }

    /// The list is the point: every one of these is a way a user or a stream
    /// can fail to say yes, and all of them must mean no.
    #[test]
    fn everything_else_declines_including_silence() {
        for answer in [
            "",
            "\n",
            " ",
            "n",
            "N",
            "no",
            "maybe",
            "yess",
            "ye",
            "Yes please",
        ] {
            assert_eq!(
                decide(Some(answer)),
                Consent::Declined,
                "{answer:?} declines"
            );
        }
        assert_eq!(decide(None), Consent::Declined);
    }
}
