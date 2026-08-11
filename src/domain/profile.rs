//! The profile document: the ordered layer list and its strategy table.
//!
//! This module is for what the profile file itself declares. It is not for
//! structural validation of the child's settings or the contributor provenance
//! map, which belong with the composition machinery.

use crate::error::DomainError;

use super::{identifier::Identifier, strategy::StrategyTable};

/// A parsed profile document.
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Profile {
    /// The ordered, non-empty piece names. Later wins at merge time.
    layers: Vec<Identifier>,
    /// The declared per-key array exceptions, empty when absent.
    #[serde(default)]
    array_strategies: StrategyTable,
}

impl Profile {
    /// Parses a profile document and rejects an empty layer list.
    pub(crate) fn parse(text: &str) -> Result<Self, DomainError> {
        let profile: Self = serde_yaml_ng::from_str(text)
            .map_err(|error| DomainError::Semantic(error.to_string()))?;
        if profile.layers.is_empty() {
            return Err(DomainError::Semantic(
                "a profile must name at least one layer".to_owned(),
            ));
        }
        Ok(profile)
    }

    /// Returns the ordered piece names.
    pub(crate) fn layers(&self) -> &[Identifier] {
        &self.layers
    }

    /// Returns the declared array merge strategies.
    pub(crate) const fn strategies(&self) -> &StrategyTable {
        &self.array_strategies
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
    fn a_strategy_table_parses_into_typed_entries() {
        let profile = Profile::parse(
            "layers:\n  - base\narray_strategies:\n  \"/permissions/allow\":\n    \
            strategy: concat\n",
        )
        .expect("parses");
        let pointer = crate::domain::pointer::Pointer::parse("/permissions/allow").expect("valid");
        assert_eq!(
            profile.strategies().get(&pointer),
            Some(&crate::domain::strategy::ArrayStrategy::Concat)
        );
    }

    /// The table is refused by the profile, not deferred to merge time: a
    /// declaration that cannot mean anything is a malformed document.
    #[test]
    fn a_profile_declaring_an_unusable_strategy_is_rejected() {
        assert!(
            Profile::parse(
                "layers:\n  - base\narray_strategies:\n  \"/a\":\n    strategy: replace\n"
            )
            .is_err()
        );
        assert!(
            Profile::parse(
                "layers:\n  - base\narray_strategies:\n  \"a/b\":\n    strategy: concat\n"
            )
            .is_err()
        );
    }

    #[test]
    fn an_absent_strategy_table_is_empty() {
        let profile = Profile::parse("layers:\n  - base\n").expect("parses");
        assert_eq!(profile.strategies().iter().count(), 0);
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
