//! Pure account metadata, selection, and report values.

use crate::domain::{config::Source, identifier::Identifier};

/// Authentication modes recognized in durable metadata.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum AuthMode {
    Login,
    Token,
}

impl AuthMode {
    pub(crate) const fn spelling(self) -> &'static str {
        match self {
            Self::Login => "login",
            Self::Token => "token",
        }
    }
}

/// Strict required metadata fields. Future token-only fields are ignored.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub(crate) struct AuthModeMetadata {
    pub(crate) mode: AuthMode,
    pub(crate) recorded_at: RecordedAt,
}

/// A validated RFC 3339 UTC timestamp.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(try_from = "String", into = "String")]
pub(crate) struct RecordedAt(String);

impl RecordedAt {
    pub(crate) fn parse(value: String) -> Result<Self, String> {
        let bytes = value.as_bytes();
        let shape = bytes.len() == 20
            && bytes[4] == b'-'
            && bytes[7] == b'-'
            && bytes[10] == b'T'
            && bytes[13] == b':'
            && bytes[16] == b':'
            && bytes[19] == b'Z'
            && bytes.iter().enumerate().all(|(index, byte)| {
                matches!(index, 4 | 7 | 10 | 13 | 16 | 19) || byte.is_ascii_digit()
            });
        if !shape {
            return Err("recorded_at must be RFC 3339 UTC seconds".into());
        }
        let number = |range: std::ops::Range<usize>| {
            std::str::from_utf8(&bytes[range]).ok()?.parse::<u32>().ok()
        };
        let (Some(year), Some(month), Some(day), Some(hour), Some(minute), Some(second)) = (
            number(0..4),
            number(5..7),
            number(8..10),
            number(11..13),
            number(14..16),
            number(17..19),
        ) else {
            return Err("recorded_at contains an invalid number".into());
        };
        let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
        let month_days = match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 if leap => 29,
            2 => 28,
            _ => 0,
        };
        if day == 0 || day > month_days || hour > 23 || minute > 59 || second > 59 {
            return Err("recorded_at contains an out-of-range component".into());
        }
        Ok(Self(value))
    }

    pub(crate) const fn new_unchecked(value: String) -> Self {
        Self(value)
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for RecordedAt {
    type Error = String;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl From<RecordedAt> for String {
    fn from(value: RecordedAt) -> Self {
        value.0
    }
}

/// The public account-selection provenance vocabulary.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
pub(crate) enum SelectionSource {
    Flag,
    Environment,
    UserConfig,
    Marker,
    None,
}

impl SelectionSource {
    pub(crate) const fn spelling(self) -> &'static str {
        match self {
            Self::Flag => "flag",
            Self::Environment => "environment",
            Self::UserConfig => "user-config",
            Self::Marker => "marker",
            Self::None => "none",
        }
    }
}

impl From<Source> for SelectionSource {
    fn from(source: Source) -> Self {
        match source {
            Source::Cli => Self::Flag,
            Source::Environment => Self::Environment,
            Source::User => Self::UserConfig,
            Source::Default | Source::Project => Self::None,
        }
    }
}

/// One immutable selection resolved before application context construction.
#[derive(Clone, Debug)]
pub(crate) struct AccountSelection {
    account: Option<Identifier>,
    source: SelectionSource,
}

impl AccountSelection {
    pub(crate) const fn none() -> Self {
        Self {
            account: None,
            source: SelectionSource::None,
        }
    }
    pub(crate) const fn new(account: Identifier, source: SelectionSource) -> Self {
        Self {
            account: Some(account),
            source,
        }
    }
    pub(crate) const fn account(&self) -> Option<&Identifier> {
        self.account.as_ref()
    }
    pub(crate) const fn source(&self) -> SelectionSource {
        self.source
    }
}

/// List-only mode projection. `Invalid` is never durable metadata.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ReportMode {
    Login,
    Token,
    Invalid,
}

impl ReportMode {
    pub(crate) const fn spelling(self) -> &'static str {
        match self {
            Self::Login => "login",
            Self::Token => "token",
            Self::Invalid => "invalid",
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct AccountFinding {
    pub(crate) name: Identifier,
    pub(crate) mode: ReportMode,
    pub(crate) usable: bool,
    pub(crate) selected: bool,
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn metadata_requires_typed_fields_but_accepts_future_token_fields() {
        let value: AuthModeMetadata = serde_json::from_str(
            r#"{"mode":"token","recorded_at":"2026-08-11T12:34:56Z","fingerprint":"sha256[..8]"}"#,
        )
        .expect("token metadata");
        assert_eq!(value.mode, AuthMode::Token);
        assert!(serde_json::from_str::<AuthModeMetadata>(r#"{"mode":"login"}"#).is_err());
        assert!(
            serde_json::from_str::<AuthModeMetadata>(
                r#"{"mode":"other","recorded_at":"2026-08-11T12:34:56Z"}"#
            )
            .is_err()
        );
    }

    #[test]
    fn invalid_is_the_pinned_list_only_spelling() {
        assert_eq!(ReportMode::Invalid.spelling(), "invalid");
    }

    #[test]
    fn sources_have_only_the_public_account_spellings() {
        assert_eq!(SelectionSource::Flag.spelling(), "flag");
        assert_eq!(SelectionSource::Environment.spelling(), "environment");
        assert_eq!(SelectionSource::UserConfig.spelling(), "user-config");
        assert_eq!(SelectionSource::Marker.spelling(), "marker");
        assert_eq!(SelectionSource::None.spelling(), "none");
        assert_eq!(
            SelectionSource::from(Source::Project),
            SelectionSource::None
        );
    }
}
