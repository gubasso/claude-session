//! The one owner of the YAML parser configuration.
//!
//! Every option that can change what a profile means is set here by name, so a
//! changed default in a later `serde-saphyr` release cannot move a parse result
//! without this file changing too. The choices and their reasons are in
//! [ADR-0127](../../docs/decisions/ADR-0127-parse-yaml-with-serde-saphyr.md).

use serde::Deserialize;
use serde_saphyr::{
    Budget, DuplicateKeyPolicy, Error, MergeKeyPolicy, NonFiniteFloatPolicy, Options,
    UserMessageFormatter,
};

/// Returns the parser options every profile parse uses.
pub(crate) fn options() -> Options {
    serde_saphyr::options! {
        // Only `true` and `false` are booleans; `yes`, `on`, and the other
        // YAML 1.1 spellings stay text.
        strict_booleans: true,
        duplicate_keys: DuplicateKeyPolicy::Error,
        // `<<` is an ordinary key, so a closed field set refuses it instead of
        // pulling fields in from another mapping.
        merge_keys: MergeKeyPolicy::AsOrdinary,
        // A `!!binary` string field keeps its text rather than being decoded.
        ignore_binary_tag_for_string: true,
        // Typed string fields ignore an application tag such as `!foo`.
        reject_unsupported_tags: false,
        legacy_octal_numbers: false,
        non_finite_float_policy: NonFiniteFloatPolicy::PassThrough,
        budget: Some(Budget::default()),
        // The diagnostic already names the profile path, and the message
        // names the line and column. A source excerpt would name its file
        // `<input>` beside that path and repeat the message.
        with_snippet: false,
    }
}

/// Parses one YAML document with [`options`].
pub(crate) fn from_str<'de, T>(text: &'de str) -> Result<T, Error>
where
    T: Deserialize<'de>,
{
    serde_saphyr::from_str_with_options(text, options())
}

/// Renders a parser error for the person who wrote the file: what is wrong
/// and where, with no advice about the parser's own API.
pub(crate) fn user_message(error: &Error) -> String {
    error.render_with_formatter(&UserMessageFormatter)
}
