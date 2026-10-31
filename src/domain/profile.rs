//! The profile document: the ordered layer list and its strategy table.
//!
//! This module is for what the profile file itself declares. It is not for
//! structural validation of the child's settings or the contributor provenance
//! map, which belong with the composition machinery.

use crate::error::DomainError;

use super::{identifier::Identifier, strategy::StrategyTable, yaml};

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
        let profile: Self = yaml::from_str(text)
            .map_err(|error| DomainError::Semantic(yaml::user_message(&error)))?;
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

    fn layer_names(text: &str) -> Vec<String> {
        Profile::parse(text)
            .expect("parses")
            .layers()
            .iter()
            .map(|layer| layer.as_str().to_owned())
            .collect()
    }

    /// A layer name is the text the author wrote. A plain scalar that YAML
    /// could also read as a boolean or a number stays that exact string, so a
    /// piece named `on` or `007` is loaded from the file of that name.
    #[test]
    fn a_layer_name_keeps_the_spelling_of_a_scalar_yaml_could_retype() {
        for spelling in ["on", "yes", "007", "1_000", "true", "12", "0xb"] {
            assert_eq!(
                layer_names(&format!("layers:\n  - {spelling}\n")),
                [spelling],
                "{spelling}"
            );
        }
        assert_eq!(layer_names("layers:\n  - \"null\"\n"), ["null"]);
    }

    /// The identifier grammar decides a layer name, whatever YAML type the
    /// scalar resembles. A spelling outside the grammar is refused.
    #[test]
    fn a_retypable_spelling_outside_the_identifier_grammar_is_rejected() {
        for spelling in ["True", "1.5", "-1", "~", ".inf"] {
            assert!(
                Profile::parse(&format!("layers:\n  - {spelling}\n")).is_err(),
                "{spelling}"
            );
        }
    }

    /// A core string tag and an application tag do not change a layer name:
    /// the field is a string, and the tag adds nothing to it.
    #[test]
    fn a_string_tag_or_an_application_tag_leaves_the_layer_name() {
        assert_eq!(layer_names("layers:\n  - !!str base\n"), ["base"]);
        assert_eq!(layer_names("layers:\n  - !foo base\n"), ["base"]);
    }

    /// An alias repeats the anchored value, so a profile may name one piece
    /// twice without writing it twice.
    #[test]
    fn an_alias_repeats_the_anchored_layer() {
        assert_eq!(
            layer_names("layers:\n  - &shared base\n  - work\n  - *shared\n"),
            ["base", "work", "base"]
        );
    }

    /// Two equal keys are an authoring error: neither one may silently win.
    #[test]
    fn a_duplicate_root_key_is_rejected() {
        assert!(Profile::parse("layers:\n  - base\nlayers:\n  - work\n").is_err());
    }

    /// The strategy table refuses a location declared twice when the profile
    /// carries it, not only when the table is parsed alone.
    #[test]
    fn a_pointer_declared_twice_in_the_profile_is_rejected() {
        assert!(
            Profile::parse(
                "layers:\n  - base\narray_strategies:\n  \"/a\":\n    strategy: concat\n  \
                \"/a\":\n    strategy: concat\n"
            )
            .is_err()
        );
    }

    /// The fields of a strategy entry are closed in a profile too.
    #[test]
    fn an_unknown_field_in_a_strategy_entry_is_rejected() {
        assert!(
            Profile::parse(
                "layers:\n  - base\narray_strategies:\n  \"/a\":\n    strategy: concat\n    \
                nope: 1\n"
            )
            .is_err()
        );
    }

    /// A merge key is an ordinary key, so the closed field set refuses it. It
    /// never pulls fields from another mapping into the profile.
    #[test]
    fn a_merge_key_at_the_root_is_rejected() {
        assert!(Profile::parse("layers:\n  - base\n<<:\n  array_strategies: {}\n").is_err());
        assert!(Profile::parse("<<:\n  layers:\n    - base\n").is_err());
    }

    /// An empty table and an absent table mean the same thing: no exceptions.
    #[test]
    fn an_empty_strategy_table_is_empty() {
        let profile = Profile::parse("layers:\n  - base\narray_strategies: {}\n").expect("parses");
        assert_eq!(profile.strategies().iter().count(), 0);
    }

    /// A profile is one mapping in one document. Anything else names no
    /// layers and is refused.
    #[test]
    fn a_document_that_is_not_one_mapping_is_rejected() {
        for text in [
            "",
            "# only a comment\n",
            "null\n",
            "~\n",
            "layers:\n  - base\n---\nlayers:\n  - work\n",
            "- base\n",
        ] {
            assert!(Profile::parse(text).is_err(), "{text:?}");
        }
    }

    /// YAML forbids a tab in indentation. A tab as the first indentation
    /// character is refused.
    #[test]
    fn a_tab_as_the_first_indentation_character_is_rejected() {
        assert!(Profile::parse("layers:\n\t- base\n").is_err());
    }

    fn message(text: &str) -> String {
        match Profile::parse(text) {
            Err(DomainError::Semantic(message)) => message,
            other => panic!("expected a semantic error, found {other:?}"),
        }
    }

    /// An unquoted `null` is the YAML null, not a name, so it cannot be a
    /// layer. A piece named `null` is written `"null"`. Before serde-saphyr
    /// the unquoted spelling was read as the layer `null`.
    #[test]
    fn an_unquoted_null_layer_is_rejected() {
        assert!(message("layers:\n  - null\n").contains("null"));
    }

    /// A null strategy table is an absent one. Before serde-saphyr,
    /// `array_strategies: null` and `array_strategies: ~` were refused.
    #[test]
    fn a_null_strategy_table_is_empty() {
        for value in ["null", "~"] {
            let profile =
                Profile::parse(&format!("layers:\n  - base\narray_strategies: {value}\n"))
                    .expect("parses");
            assert_eq!(profile.strategies().iter().count(), 0, "{value}");
        }
    }

    /// A core tag that names another type does not fit a string field. Before
    /// serde-saphyr the tag was ignored and the text became the layer name.
    #[test]
    fn a_core_tag_of_another_type_on_a_layer_is_rejected() {
        for tag in ["!!int", "!!bool", "!!null", "!!seq"] {
            assert!(
                Profile::parse(&format!("layers:\n  - {tag} base\n")).is_err(),
                "{tag}"
            );
        }
    }

    /// The default parse budget caps how often one anchor is replayed, which
    /// bounds the work an alias-heavy file can cause. 99 aliases of one anchor
    /// pass and 100 are refused. Before serde-saphyr there was no such cap.
    #[test]
    fn one_hundred_aliases_of_one_anchor_are_rejected() {
        let profile =
            |aliases: usize| format!("layers:\n  - &a base\n{}", "  - *a\n".repeat(aliases));
        assert_eq!(layer_names(&profile(99)).len(), 100);
        assert!(Profile::parse(&profile(100)).is_err());
    }

    /// A missing field inside a mapping names its place. An empty explicit
    /// document has no mapping to point at, so its message has no place.
    /// Before serde-saphyr it was the other way round: the empty document
    /// reported a line and column, and the empty mapping reported none.
    #[test]
    fn a_missing_layer_list_names_a_place_only_inside_a_mapping() {
        assert!(message("{}\n").contains("line 1"), "{}", message("{}\n"));
        assert!(!message("---\n").contains("line"), "{}", message("---\n"));
    }

    /// The parser reads the YAML 1.2 grammar: a tab after the indentation
    /// spaces, a tab after `-`, and a directive it does not know are allowed.
    /// Before serde-saphyr each of these was a parse error.
    #[test]
    fn the_yaml_1_2_grammar_is_accepted() {
        for text in [
            "layers:\n  \t- base\n",
            "layers:\n  -\tbase\n",
            "%FOO bar\n---\nlayers:\n  - base\n",
            "%FOO\n---\nlayers:\n  - base\n",
            "%YAML 1.3\n---\nlayers:\n  - base\n",
        ] {
            assert_eq!(layer_names(text), ["base"], "{text:?}");
        }
    }
}
