//! The per-key array merge strategy table a profile declares.
//!
//! The table's whole grammar is `configuration.md#declaring-a-strategy`: two
//! spellings, one optional field, and four rejections. Everything it refuses is
//! refused here, at parse time, so the merge engine never has to ask whether an
//! entry made sense.

use std::collections::BTreeMap;

use serde::{
    Deserialize,
    de::{self, MapAccess, Visitor},
};

use super::pointer::Pointer;

/// What to do with an array whose location the table names.
///
/// `replace` is deliberately absent: it is what an unlisted array already does,
/// so an entry declaring it would discriminate nothing
/// ([ADR-0051](../../docs/decisions/ADR-0051-let-every-surface-element-discriminate.md)).
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ArrayStrategy {
    /// Elements appended in layer order.
    Concat,
    /// Elements matched on a named field and merged; unmatched ones appended.
    MergeByKey { key: String },
}

impl ArrayStrategy {
    /// Returns the spelling a profile writes, which reports also print.
    pub(crate) const fn spelling(&self) -> &'static str {
        match *self {
            Self::Concat => "concat",
            Self::MergeByKey { .. } => "merge-by-key",
        }
    }
}

/// Every declared exception, ordered by pointer.
///
/// A `BTreeMap` because the sidecar's key order is part of its shape, and an
/// insertion-ordered map would make two equivalent profiles produce two
/// different sidecars.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct StrategyTable(BTreeMap<Pointer, ArrayStrategy>);

impl StrategyTable {
    /// Returns the strategy declared for one location, if any.
    pub(crate) fn get(&self, pointer: &Pointer) -> Option<&ArrayStrategy> {
        self.0.get(pointer)
    }

    /// Iterates the declarations in pointer order.
    pub(crate) fn iter(&self) -> impl Iterator<Item = (&Pointer, &ArrayStrategy)> {
        self.0.iter()
    }
}

/// One table row, before its cross-field rules are applied.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    strategy: String,
    #[serde(default)]
    key: Option<String>,
}

impl Entry {
    /// Applies the rule table to one row.
    fn into_strategy(self, pointer: &str) -> Result<ArrayStrategy, String> {
        match self.strategy.as_str() {
            "concat" => {
                if self.key.is_some() {
                    return Err(format!(
                        "the strategy for {pointer} is concat, which takes no key"
                    ));
                }
                Ok(ArrayStrategy::Concat)
            }
            "merge-by-key" => match self.key {
                Some(key) if !key.is_empty() => Ok(ArrayStrategy::MergeByKey { key }),
                _ => Err(format!(
                    "the strategy for {pointer} is merge-by-key, which requires one non-empty key"
                )),
            },
            "replace" => Err(format!(
                "the strategy for {pointer} is replace, which is what an unlisted array \
                already does; remove the entry"
            )),
            other => Err(format!(
                "the strategy for {pointer} is {other}, which is not concat or merge-by-key"
            )),
        }
    }
}

impl<'de> Deserialize<'de> for StrategyTable {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct TableVisitor;

        impl<'de> Visitor<'de> for TableVisitor {
            type Value = StrategyTable;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a map of JSON pointers to array merge strategies")
            }

            // Hand-written rather than derived, because serde silently keeps the
            // last of two identical mapping keys. A table that listed one
            // location twice would then apply a strategy the author did not
            // write, with nothing anywhere reporting it.
            fn visit_map<M>(self, mut access: M) -> Result<StrategyTable, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut table = BTreeMap::new();
                while let Some((text, entry)) = access.next_entry::<String, Entry>()? {
                    let pointer = Pointer::parse(&text).map_err(de::Error::custom)?;
                    let strategy = entry.into_strategy(&text).map_err(de::Error::custom)?;
                    if table.insert(pointer, strategy).is_some() {
                        return Err(de::Error::custom(format!(
                            "the JSON pointer {text} is declared more than once"
                        )));
                    }
                }
                Ok(StrategyTable(table))
            }
        }

        deserializer.deserialize_map(TableVisitor)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    fn parse(yaml: &str) -> Result<StrategyTable, serde_saphyr::Error> {
        crate::domain::yaml::from_str(yaml)
    }

    #[test]
    fn a_concat_entry_parses() {
        let table = parse("\"/permissions/allow\":\n  strategy: concat\n").expect("parses");
        let pointer = Pointer::parse("/permissions/allow").expect("valid");
        assert_eq!(table.get(&pointer), Some(&ArrayStrategy::Concat));
    }

    #[test]
    fn a_merge_by_key_entry_requires_a_non_empty_key() {
        let table = parse("\"/hooks/PreToolUse\":\n  strategy: merge-by-key\n  key: matcher\n")
            .expect("parses");
        let pointer = Pointer::parse("/hooks/PreToolUse").expect("valid");
        assert_eq!(
            table.get(&pointer),
            Some(&ArrayStrategy::MergeByKey {
                key: "matcher".to_owned()
            })
        );
        assert!(parse("\"/a\":\n  strategy: merge-by-key\n  key: \"\"\n").is_err());
    }

    #[test]
    fn a_merge_by_key_entry_without_a_key_is_rejected() {
        assert!(parse("\"/a\":\n  strategy: merge-by-key\n").is_err());
    }

    #[test]
    fn a_concat_entry_with_a_key_is_rejected() {
        assert!(parse("\"/a\":\n  strategy: concat\n  key: matcher\n").is_err());
    }

    #[test]
    fn an_explicit_replace_is_rejected() {
        assert!(parse("\"/a\":\n  strategy: replace\n").is_err());
    }

    #[test]
    fn a_non_canonical_pointer_is_rejected() {
        assert!(parse("\"a/b\":\n  strategy: concat\n").is_err());
        assert!(parse("\"/a~b\":\n  strategy: concat\n").is_err());
    }

    #[test]
    fn a_duplicate_pointer_is_rejected() {
        assert!(
            parse("? \"/a\"\n: { strategy: concat }\n? \"/a\"\n: { strategy: concat }\n").is_err()
        );
    }

    /// Declaration order must not reach the output: the table is read in
    /// pointer order so two spellings of one profile produce one sidecar.
    #[test]
    fn declaration_order_is_stable() {
        let first =
            parse("\"/z\":\n  strategy: concat\n\"/a\":\n  strategy: concat\n").expect("parses");
        let second =
            parse("\"/a\":\n  strategy: concat\n\"/z\":\n  strategy: concat\n").expect("parses");
        let order: Vec<&str> = first.iter().map(|(pointer, _)| pointer.as_str()).collect();
        assert_eq!(order, ["/a", "/z"]);
        assert_eq!(first, second);
    }

    #[test]
    fn an_unknown_row_field_is_rejected() {
        assert!(parse("\"/a\":\n  strategy: concat\n  nope: 1\n").is_err());
    }

    fn merge_key_of(yaml: &str) -> String {
        let table = parse(yaml).expect("parses");
        let pointer = Pointer::parse("/a").expect("valid");
        match table.get(&pointer) {
            Some(ArrayStrategy::MergeByKey { key }) => key.clone(),
            other => panic!("expected merge-by-key, found {other:?}"),
        }
    }

    /// A merge key names a field of the child's array elements, so it is the
    /// text the author wrote. A plain scalar that YAML could also read as a
    /// boolean or a number stays that exact string.
    #[test]
    fn a_merge_key_keeps_the_spelling_of_a_scalar_yaml_could_retype() {
        for spelling in ["on", "yes", "007", "1_000", "true", "12", "0xb"] {
            assert_eq!(
                merge_key_of(&format!(
                    "\"/a\":\n  strategy: merge-by-key\n  key: {spelling}\n"
                )),
                spelling,
                "{spelling}"
            );
        }
        assert_eq!(
            merge_key_of("\"/a\":\n  strategy: merge-by-key\n  key: \"null\"\n"),
            "null"
        );
    }

    /// A string tag, an application tag, and a binary tag leave the text of a
    /// merge key as written. The binary tag is not decoded.
    #[test]
    fn a_tag_on_a_merge_key_leaves_its_text() {
        for (tagged, text) in [
            ("!!str matcher", "matcher"),
            ("!foo matcher", "matcher"),
            ("!!binary aGVsbG8=", "aGVsbG8="),
        ] {
            assert_eq!(
                merge_key_of(&format!(
                    "\"/a\":\n  strategy: merge-by-key\n  key: {tagged}\n"
                )),
                text,
                "{tagged}"
            );
        }
    }

    /// An alias repeats an anchored entry, so two locations can share one
    /// declaration.
    #[test]
    fn an_alias_repeats_an_anchored_entry() {
        let table = parse("\"/a\": &row\n  strategy: concat\n\"/b\": *row\n").expect("parses");
        for location in ["/a", "/b"] {
            let pointer = Pointer::parse(location).expect("valid");
            assert_eq!(
                table.get(&pointer),
                Some(&ArrayStrategy::Concat),
                "{location}"
            );
        }
    }

    /// A merge key is an ordinary key. In an entry it is an unknown field, and
    /// in the table it is not a JSON pointer, so both are refused. It never
    /// copies a declaration from another mapping.
    #[test]
    fn a_merge_key_is_rejected_in_an_entry_and_in_the_table() {
        assert!(parse("\"/a\":\n  <<: { strategy: concat }\n").is_err());
        assert!(parse("<<:\n  \"/a\": { strategy: concat }\n").is_err());
    }

    /// A location is a JSON pointer, which is text. A key that is a sequence
    /// or a mapping is refused.
    #[test]
    fn a_non_string_table_key_is_rejected() {
        assert!(parse("? [\"/a\"]\n: { strategy: concat }\n").is_err());
        assert!(parse("? { a: 1 }\n: { strategy: concat }\n").is_err());
    }

    /// A core tag that names another type does not fit the string field of a
    /// merge key. Before serde-saphyr the tag was ignored and the text kept.
    #[test]
    fn a_core_tag_of_another_type_on_a_merge_key_is_rejected() {
        for tag in ["!!int", "!!bool", "!!null", "!!seq"] {
            assert!(
                parse(&format!(
                    "\"/a\":\n  strategy: merge-by-key\n  key: {tag} matcher\n"
                ))
                .is_err(),
                "{tag}"
            );
        }
    }

    /// A missing field inside an entry still names its place.
    #[test]
    fn a_missing_strategy_names_its_place() {
        let error = parse("\"/a\": {}\n").expect_err("refused");
        assert!(error.location().is_some(), "{error}");
    }
}
