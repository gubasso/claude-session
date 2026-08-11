//! The child settings keys this wrapper recognizes.
//!
//! Shallow on purpose: top level only, no nesting and no types. It exists to
//! answer "have I seen this name before" so an unrecognized key can be reported
//! with provenance, and nothing else. Rejecting an unrecognized key is out of
//! scope and gated by `Q-003`.

use serde_json::Value;

/// The child settings keys this wrapper recognizes at the document root.
///
/// Owner: `docs/reference/configuration.md#validation`, which carries the same
/// list and the reason it stays shallow. Derived from the keys that page and
/// `docs/reference/accounts.md` already name — not from the child's published
/// reference, which documents many more. That gap is `Q-009` in
/// `docs/plan/open-questions.md`, and until it exits this table will warn on
/// settings the child accepts. Sorted and unique, which
/// `the_table_is_sorted_and_unique` enforces.
pub(crate) const KNOWN_TOP_LEVEL: &[&str] = &[
    "apiKeyHelper",
    "awsAuthRefresh",
    "awsCredentialExport",
    "cleanupPeriodDays",
    "env",
    "forceLoginMethod",
    "hooks",
    "includeCoAuthoredBy",
    "model",
    "outputStyle",
    "permissions",
    "sandbox",
    "statusLine",
    "statusLineCommand",
];

/// Returns the composed document's top-level keys that are not in the table.
///
/// Depth is one: a nested key is never reported, because the wrapper does not
/// claim to know the shape underneath a recognized name.
pub(crate) fn unknown_top_level(document: &Value) -> Vec<&str> {
    document.as_object().map_or_else(Vec::new, |members| {
        members
            .keys()
            .map(String::as_str)
            .filter(|key| KNOWN_TOP_LEVEL.binary_search(key).is_err())
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The lookup is a binary search, so sortedness is correctness rather than
    /// tidiness, and a duplicate would hide a name behind itself.
    #[test]
    fn the_table_is_sorted_and_unique() {
        let mut sorted = KNOWN_TOP_LEVEL.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted, KNOWN_TOP_LEVEL);
    }

    #[test]
    fn a_document_of_known_keys_reports_nothing() {
        let document = serde_json::json!({ "model": "sonnet", "env": { "A": "1" } });
        assert!(unknown_top_level(&document).is_empty());
    }

    #[test]
    fn an_unknown_root_key_is_reported() {
        let document = serde_json::json!({ "model": "sonnet", "zzzNotAKey": true });
        assert_eq!(unknown_top_level(&document), ["zzzNotAKey"]);
    }

    #[test]
    fn a_nested_key_is_never_reported() {
        let document = serde_json::json!({ "permissions": { "zzzNotAKey": [] } });
        assert!(unknown_top_level(&document).is_empty());
    }
}
