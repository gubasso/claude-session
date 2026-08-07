//! The profile document, as far as the entry key needs it.
//!
//! This module is for the ordered layer list the input digest ranges over. It
//! is not for merge strategies, structural validation, or the contributor
//! provenance map, all of which belong with the composition machinery.

use crate::error::DomainError;

use super::identifier::Identifier;

/// A parsed profile document.
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Profile {
    /// The ordered, non-empty piece names. Later wins at merge time.
    layers: Vec<Identifier>,
    // Modelled and never interpreted. `deny_unknown_fields` would otherwise
    // reject a valid profile that declares one, and the table still reaches the
    // entry key: it is a field of this file, so the file's own content digest
    // covers it.
    #[serde(default)]
    #[allow(
        dead_code,
        reason = "the digest covers the table through the file bytes"
    )]
    array_strategies: Option<serde_yaml_ng::Value>,
}

impl Profile {
    /// Parses a profile document and rejects an empty layer list.
    pub(crate) fn parse(text: &str) -> Result<Self, DomainError> {
        let profile: Self = serde_yaml_ng::from_str(text)
            .map_err(|error| DomainError::InvalidArguments(error.to_string()))?;
        if profile.layers.is_empty() {
            return Err(DomainError::InvalidArguments(
                "a profile must name at least one layer".to_owned(),
            ));
        }
        Ok(profile)
    }

    /// Returns the ordered piece names.
    pub(crate) fn layers(&self) -> &[Identifier] {
        &self.layers
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn the_layer_list_keeps_its_order() {
        let profile = Profile::parse("layers:\n  - base\n  - work\n").expect("parses");
        let names: Vec<&str> = profile.layers().iter().map(Identifier::as_str).collect();
        assert_eq!(names, ["base", "work"]);
    }

    #[test]
    fn a_strategy_table_parses_without_being_interpreted() {
        let profile = Profile::parse(
            "layers:\n  - base\narray_strategies:\n  \"/permissions/allow\":\n    \
            strategy: concat\n",
        )
        .expect("parses");
        assert_eq!(profile.layers().len(), 1);
    }

    #[test]
    fn an_empty_layer_list_is_rejected() {
        assert!(Profile::parse("layers: []\n").is_err());
    }

    #[test]
    fn an_unknown_field_is_rejected() {
        assert!(Profile::parse("layers:\n  - base\nnope: 1\n").is_err());
    }

    #[test]
    fn a_layer_violating_the_identifier_grammar_is_rejected() {
        assert!(Profile::parse("layers:\n  - Base\n").is_err());
        assert!(Profile::parse("layers:\n  - a/b\n").is_err());
    }
}
