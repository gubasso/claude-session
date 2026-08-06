//! Validated identifiers used by wrapper configuration.

use std::{fmt, str::FromStr};

use crate::error::DomainError;

/// A lowercase, path-safe account or profile identifier.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize)]
#[serde(try_from = "String")]
pub(crate) struct Identifier(String);

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

impl fmt::Display for Identifier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}
