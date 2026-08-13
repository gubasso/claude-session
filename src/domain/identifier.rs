//! Validated identifiers used by wrapper configuration.

use std::{fmt, str::FromStr};

use crate::error::DomainError;

/// A lowercase, path-safe account or profile identifier.
/// Serialization is the durable-record direction only: the grammar is checked
/// on the way in, so writing one back out cannot produce a value the reader
/// would refuse. Report documents still print through [`Identifier::as_str`].
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(try_from = "String", into = "String")]
pub(crate) struct Identifier(String);

impl From<Identifier> for String {
    fn from(value: Identifier) -> Self {
        value.0
    }
}

impl FromStr for Identifier {
    type Err = DomainError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let bytes = value.as_bytes();
        let first_valid = bytes.first().is_some_and(u8::is_ascii_lowercase)
            || bytes.first().is_some_and(u8::is_ascii_digit);
        let rest_valid = bytes.get(1..).is_some_and(|rest| {
            rest.iter().all(|byte| {
                byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-')
            })
        });
        if !(1..=32).contains(&bytes.len()) || !first_valid || !rest_valid {
            return Err(DomainError::InvalidIdentifier(value.to_owned()));
        }
        Ok(Self(value.to_owned()))
    }
}

impl TryFrom<String> for Identifier {
    type Error = DomainError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl Identifier {
    /// Borrows the validated identifier text.
    ///
    /// Path construction joins this directly. Going through `Display` would
    /// allocate a `String` for every component of every managed path, on a
    /// value whose grammar already guarantees it is one safe component.
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Identifier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    fn parse(value: &str) -> Result<Identifier, DomainError> {
        value.parse()
    }

    #[test]
    fn the_documented_character_set_is_accepted() {
        for value in ["a", "0", "z9", "a-b_c", "work", &"x".repeat(32)] {
            assert!(parse(value).is_ok(), "{value} should parse");
        }
    }

    /// The grammar's first byte rule is separate from its rest rule, so a
    /// separator or a digit-only case has to be tested on both ends.
    #[test]
    fn a_leading_separator_is_rejected() {
        assert!(parse("-a").is_err());
        assert!(parse("_a").is_err());
    }

    #[test]
    fn any_uppercase_byte_is_rejected() {
        assert!(parse("A").is_err());
        assert!(parse("aB").is_err());
    }

    #[test]
    fn a_path_or_whitespace_byte_is_rejected() {
        for value in ["a.b", "a/b", "a b", "a\tb", "..", "/"] {
            assert!(parse(value).is_err(), "{value} should not parse");
        }
    }

    /// The length rule counts bytes, not characters, so a multi-byte value
    /// has to fail on the character class rather than slip through on length.
    #[test]
    fn a_non_ascii_value_is_rejected() {
        assert!(parse("á").is_err());
        assert!(parse("aá").is_err());
    }

    #[test]
    fn an_empty_value_is_rejected() {
        assert!(parse("").is_err());
    }

    #[test]
    fn a_thirty_third_byte_is_rejected() {
        assert!(parse(&"x".repeat(32)).is_ok());
        assert!(parse(&"x".repeat(33)).is_err());
    }

    /// Nothing is truncated or rewritten: the refusal carries back exactly what
    /// the user supplied, which is what lets a diagnostic name the real value.
    #[test]
    fn a_rejection_carries_the_original_value_verbatim() {
        let Err(DomainError::InvalidIdentifier(value)) = parse("Has.Dot") else {
            panic!("expected an identifier rejection");
        };
        assert_eq!(value, "Has.Dot");
    }
}
