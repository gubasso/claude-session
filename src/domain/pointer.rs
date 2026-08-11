//! RFC 6901 JSON Pointers in canonical form.
//!
//! A pointer rather than a dotted path, because the child renders nested
//! settings keys dotted and a dotted key stops being addressable the moment a
//! settings key contains a dot, while `~0` and `~1` leave nothing ambiguous
//! (`configuration.md#declaring-a-strategy`).

use std::fmt;

use crate::error::DomainError;

/// One [RFC 6901](https://www.rfc-editor.org/rfc/rfc6901.html) JSON Pointer, in
/// canonical form.
///
/// Canonical means the stored text is the only spelling of the location it
/// addresses. Two spellings of one location would let a strategy table list the
/// same key twice without appearing to, so the encoding is round-tripped at
/// parse time and a non-canonical one is refused.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct Pointer(String);

impl Pointer {
    /// Parses a pointer and rejects any non-canonical spelling.
    pub(crate) fn parse(text: &str) -> Result<Self, DomainError> {
        if text.is_empty() {
            return Ok(Self(String::new()));
        }
        if !text.starts_with('/') {
            return Err(DomainError::Semantic(format!(
                "the JSON pointer {text} must be empty or begin with a slash"
            )));
        }
        let tokens = decode(text)?;
        let reencoded = encode(&tokens);
        if reencoded == text {
            Ok(Self(text.to_owned()))
        } else {
            Err(DomainError::Semantic(format!(
                "the JSON pointer {text} is not canonical; write it as {reencoded}"
            )))
        }
    }

    /// Returns the decoded reference tokens, in order.
    pub(crate) fn tokens(&self) -> Vec<String> {
        // Infallible: the stored text was decoded once at parse time.
        decode(&self.0).unwrap_or_default()
    }

    /// Returns the canonical text.
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    /// Builds a pointer from already-decoded tokens.
    pub(crate) fn from_tokens(tokens: &[String]) -> Self {
        Self(encode(tokens))
    }
}

impl fmt::Display for Pointer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Splits on `/` and decodes `~1` then `~0`, in that order.
///
/// The order is RFC 6901's own and is not interchangeable: decoding `~0` first
/// would turn `~01` into `~1` and then into `/`, inventing a separator.
fn decode(text: &str) -> Result<Vec<String>, DomainError> {
    if text.is_empty() {
        return Ok(Vec::new());
    }
    let mut tokens = Vec::new();
    for raw in text.split('/').skip(1) {
        let mut token = String::with_capacity(raw.len());
        let mut characters = raw.chars();
        while let Some(character) = characters.next() {
            if character == '~' {
                match characters.next() {
                    Some('0') => token.push('~'),
                    Some('1') => token.push('/'),
                    Some(other) => {
                        return Err(DomainError::Semantic(format!(
                            "the JSON pointer {text} contains the unknown escape ~{other}"
                        )));
                    }
                    None => {
                        return Err(DomainError::Semantic(format!(
                            "the JSON pointer {text} ends with a dangling tilde"
                        )));
                    }
                }
            } else {
                token.push(character);
            }
        }
        tokens.push(token);
    }
    Ok(tokens)
}

/// Encodes tokens back into canonical pointer text.
fn encode(tokens: &[String]) -> String {
    let mut out = String::new();
    for token in tokens {
        out.push('/');
        for character in token.chars() {
            match character {
                '~' => out.push_str("~0"),
                '/' => out.push_str("~1"),
                other => out.push(other),
            }
        }
    }
    out
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    /// It parses because RFC 6901 defines it; the strategy table refuses it
    /// later, because the whole document is not an array.
    #[test]
    fn the_empty_pointer_addresses_the_whole_document() {
        let pointer = Pointer::parse("").expect("the empty pointer is valid");
        assert_eq!(pointer.as_str(), "");
        assert!(pointer.tokens().is_empty());
    }

    #[test]
    fn a_pointer_must_open_with_a_slash() {
        assert!(Pointer::parse("permissions/allow").is_err());
    }

    #[test]
    fn the_escapes_decode_to_a_slash_and_a_tilde() {
        let pointer = Pointer::parse("/a~1b/c~0d").expect("valid escapes");
        assert_eq!(pointer.tokens(), ["a/b".to_owned(), "c~d".to_owned()]);
    }

    #[test]
    fn a_dangling_tilde_is_not_canonical() {
        assert!(Pointer::parse("/a~").is_err());
    }

    #[test]
    fn an_unknown_escape_is_not_canonical() {
        assert!(Pointer::parse("/a~2b").is_err());
    }

    /// A raw tilde is a legal character that has a canonical escaped spelling,
    /// so accepting both would let one location be listed twice.
    #[test]
    fn a_non_canonical_encoding_is_rejected() {
        assert!(Pointer::parse("/a~b").is_err());
    }

    #[test]
    fn tokens_preserve_order_and_empty_segments() {
        let pointer = Pointer::parse("/a//b").expect("an empty token is legal");
        assert_eq!(
            pointer.tokens(),
            ["a".to_owned(), String::new(), "b".to_owned()]
        );
        assert_eq!(Pointer::from_tokens(&pointer.tokens()), pointer);
    }

    #[test]
    fn ordering_follows_the_canonical_text() {
        let mut pointers = [
            Pointer::parse("/z").expect("valid"),
            Pointer::parse("/a").expect("valid"),
        ];
        pointers.sort();
        assert_eq!(pointers[0].as_str(), "/a");
    }
}
