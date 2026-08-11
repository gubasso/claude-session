//! Wrapper-owned credential material, and the one value derived from it.
//!
//! The redaction rule in logging and output binds every human line, prompt,
//! JSON document, log record, and diagnostic. A helper that must be called to
//! work fails silently when someone forgets to call it, so the rule is carried
//! by this type instead: `Secret` has no `Display`, no `Serialize`, and no
//! conversion to a string, and the one function that turns a secret into an
//! emittable value takes a `&Secret` it is impossible to build from a
//! child-owned path.

use std::fmt;

use sha2::{Digest as _, Sha256};

use crate::domain::entry::hex;

/// The largest credential the wrapper will accept or read back.
///
/// A bound is needed because both ingest sources are streams someone else
/// fills. The value is far above any subscription token and far below anything
/// that would matter as an allocation.
const MAXIMUM: usize = 8 * 1024;

/// A wrapper-owned credential value held in memory.
///
/// Deliberately absent: `Display`, `Serialize`, `Deref`, `AsRef<[u8]>`, and any
/// constructor from a path. Reading the bytes is [`Secret::expose`], which is
/// one grep-able name rather than an implicit coercion at an emission site.
pub(crate) struct Secret(Vec<u8>);

/// Why one ingested line was refused.
///
/// A reason rather than a message, because the caller owns the diagnostic's
/// `where` clause — the pasted line and standard input are different places —
/// and because a value carrying no borrowed input cannot accidentally quote the
/// secret it rejected.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SecretError {
    /// Nothing, or only whitespace, was supplied.
    Empty,
    /// More than one line was supplied.
    MultipleLines,
    /// A control byte appeared inside the value.
    ControlByte,
    /// The value exceeded the accepted length.
    TooLong,
    /// The person entering it interrupted the prompt.
    ///
    /// Reachable only from the terminal, where the interrupt key arrives as
    /// data rather than as a signal so that the terminal can be restored before
    /// anything reports. It is a refusal like the others: nothing was supplied,
    /// so nothing is stored.
    Cancelled,
}

impl SecretError {
    /// Returns the diagnostic's `why` clause, which never quotes the input.
    pub(crate) const fn reason(self) -> &'static str {
        match self {
            Self::Empty => "no token was supplied",
            Self::MultipleLines => "a token is one line, and more than one arrived",
            Self::ControlByte => "the token contains a control character",
            Self::TooLong => "the token is longer than this wrapper accepts",
            Self::Cancelled => "entry was cancelled at the prompt",
        }
    }
}

impl Secret {
    /// Parses one ingested line.
    ///
    /// Surrounding ASCII whitespace is trimmed, because a paste carries the
    /// terminal's newline and a here-string carries its own. Nothing else is
    /// interpreted: no prefix is parsed and no lifetime is inferred from the
    /// value, so a change in the provider's token format cannot make this
    /// refuse a working credential.
    pub(crate) fn parse_line(bytes: &[u8]) -> Result<Self, SecretError> {
        if bytes.len() > MAXIMUM {
            return Err(SecretError::TooLong);
        }
        let start = bytes
            .iter()
            .position(|byte| !byte.is_ascii_whitespace())
            .ok_or(SecretError::Empty)?;
        let end = bytes
            .iter()
            .rposition(|byte| !byte.is_ascii_whitespace())
            .ok_or(SecretError::Empty)?;
        let trimmed = &bytes[start..=end];
        if trimmed.iter().any(|byte| matches!(byte, b'\n' | b'\r')) {
            return Err(SecretError::MultipleLines);
        }
        if trimmed.iter().any(|byte| *byte < 0x20 || *byte == 0x7f) {
            return Err(SecretError::ControlByte);
        }
        Ok(Self(trimmed.to_vec()))
    }

    /// Wraps bytes read back from the wrapper's own token file.
    ///
    /// The file holds the value and nothing else, so this trims nothing: the
    /// bytes written are the bytes hashed, which is what lets one fingerprint
    /// describe both the token and the file without a trimming policy that
    /// could drift between the writer and this reader.
    pub(crate) const fn from_stored(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    /// How many bytes an ingest reader may take from its stream.
    ///
    /// One byte past [`MAXIMUM`], which is the least that lets [`parse_line`]
    /// tell a value at the limit from one over it. The bound belongs on the
    /// stream and not only on the buffer it filled: both ingest sources are
    /// filled by someone else, so a reader that consumed to end of file would
    /// grow the allocation without limit and reach the length check only after
    /// an endless pipe had already been held in memory.
    ///
    /// [`parse_line`]: Self::parse_line
    pub(crate) const INGEST_BOUND: u64 = MAXIMUM as u64 + 1;

    /// Returns the credential bytes.
    ///
    /// Two call sites are legitimate: writing the token file, and placing the
    /// value in the child's environment. Anything else is a leak.
    pub(crate) fn expose(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Debug for Secret {
    /// Prints a placeholder rather than refusing to exist.
    ///
    /// Omitting `Debug` entirely would make `#[derive(Debug)]` on a containing
    /// struct fail to compile, and the shortest repair for that is a call to
    /// `expose`. A placeholder keeps the derive both compiling and safe.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Secret(<redacted>)")
    }
}

impl Drop for Secret {
    /// Overwrites the buffer, as a courtesy rather than a guarantee.
    ///
    /// The optimizer may elide this, nothing pins the page out of swap, and by
    /// the time it runs the same bytes exist in the child's environment block
    /// and in the token file. `unsafe_code` is forbidden here, so a fenced wipe
    /// is out of reach and a zeroizing dependency is not in the reviewed set.
    /// Claiming more than this would be the lie; the buffer is sized once at
    /// construction so no reallocation leaves a copy behind.
    fn drop(&mut self) {
        self.0.fill(0);
    }
}

/// The eight leading hexadecimal characters of a wrapper-owned token's digest.
///
/// This is the one secret-derived value the output rules permit, and it is
/// permitted only for a token the wrapper owns. [`Fingerprint::of`] takes a
/// `&Secret`, and no `Secret` can be constructed from a child-owned credential
/// path, so the prohibited fingerprint is unexpressible rather than merely
/// discouraged.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(try_from = "String", into = "String")]
pub(crate) struct Fingerprint(String);

/// The rendered width, which is also what a stored value must match.
const FINGERPRINT_WIDTH: usize = 8;

impl Fingerprint {
    /// Computes the fingerprint of a wrapper-owned token.
    pub(crate) fn of(secret: &Secret) -> Self {
        let digest: [u8; 32] = Sha256::digest(secret.expose()).into();
        let mut rendered = hex(&digest);
        rendered.truncate(FINGERPRINT_WIDTH);
        Self(rendered)
    }

    /// Validates a fingerprint read back from metadata.
    pub(crate) fn parse(value: String) -> Result<Self, String> {
        if value.len() == FINGERPRINT_WIDTH
            && value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            Ok(Self(value))
        } else {
            Err("fingerprint must be eight lowercase hexadecimal characters".into())
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Fingerprint {
    type Error = String;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl From<Fingerprint> for String {
    fn from(value: Fingerprint) -> Self {
        value.0
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn one_pasted_line_survives_its_surrounding_whitespace() {
        let secret = Secret::parse_line(b"  sk-value \n").expect("a trimmed line parses");
        assert_eq!(secret.expose(), b"sk-value");
    }

    /// The rejections are separate reasons because they have separate repairs,
    /// and because a reader that took the first line would accept the two-line
    /// case silently.
    #[test]
    fn every_refused_shape_names_its_own_reason() {
        let cases: [(&[u8], SecretError); 6] = [
            (b"", SecretError::Empty),
            (b"   \n\t ", SecretError::Empty),
            (b"first\nsecond\n", SecretError::MultipleLines),
            (b"value\r\nother", SecretError::MultipleLines),
            (b"va\x00lue", SecretError::ControlByte),
            (b"va\x7flue", SecretError::ControlByte),
        ];
        for (input, expected) in cases {
            assert_eq!(
                Secret::parse_line(input).err(),
                Some(expected),
                "input {input:?} must be refused"
            );
        }
        let long = vec![b'a'; MAXIMUM + 1];
        assert_eq!(Secret::parse_line(&long).err(), Some(SecretError::TooLong));
        assert!(Secret::parse_line(&long[..MAXIMUM]).is_ok());
    }

    #[test]
    fn a_secret_never_renders_its_own_bytes() {
        let secret = Secret::parse_line(b"sk-not-in-the-output").expect("a line parses");
        let rendered = format!("{secret:?}");
        assert_eq!(rendered, "Secret(<redacted>)");
        assert!(!rendered.contains("sk-not"));
    }

    #[test]
    fn a_fingerprint_is_eight_lowercase_hexadecimal_characters() {
        let secret = Secret::parse_line(b"token").expect("a line parses");
        let fingerprint = Fingerprint::of(&secret);
        // An independently computable vector, so a changed preimage, a changed
        // hash, or a changed width fails here rather than silently renaming a
        // credential: `printf token | sha256sum` begins with these characters.
        assert_eq!(fingerprint.as_str(), "3c469e9d");
        assert_eq!(Fingerprint::parse("3c469e9d".into()), Ok(fingerprint));
    }

    #[test]
    fn a_malformed_stored_fingerprint_is_refused() {
        for value in ["3C469E9D", "3c469e9", "3c469e9da", "3c469e9g"] {
            assert!(
                Fingerprint::parse(value.into()).is_err(),
                "{value} must be refused"
            );
        }
    }
}
