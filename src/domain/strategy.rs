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

    fn parse(yaml: &str) -> Result<StrategyTable, serde_yaml_ng::Error> {
        serde_yaml_ng::from_str(yaml)
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
}
