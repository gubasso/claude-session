//! The ordered fold that turns settings pieces into one document.
//!
//! Pure, and it takes piece indices rather than names, so `domain` never needs
//! a service type: the caller maps an index back to the name it reports. The
//! rules are `configuration.md#merge-semantics` and nothing beyond it.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use super::{pointer::Pointer, strategy::ArrayStrategy, strategy::StrategyTable};

/// Where one composed leaf came from.
///
/// Leaves only. A merged array is one entry, and an object container is not
/// itself an entry: it would duplicate every descendant while explaining
/// nothing the descendants do not already say.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct KeyProvenance {
    /// The piece whose value survived, or the last contributor to a merge.
    pub(crate) piece: usize,
    /// Every earlier piece that set this key and lost, in layer order.
    pub(crate) overrode: Vec<usize>,
    /// The strategy applied, when the key is a merged array.
    pub(crate) strategy: Option<ArrayStrategy>,
    /// Every piece that contributed, when the key is a merged array.
    pub(crate) contributors: Vec<usize>,
}

impl KeyProvenance {
    /// A key exactly one piece wrote.
    const fn single(piece: usize) -> Self {
        Self {
            piece,
            overrode: Vec::new(),
            strategy: None,
            contributors: Vec::new(),
        }
    }
}

/// One composed document and the account of how it got that way.
#[derive(Clone, Debug)]
pub(crate) struct Composed {
    /// The folded document.
    pub(crate) document: Value,
    /// Per-leaf provenance, in pointer order.
    pub(crate) keys: BTreeMap<Pointer, KeyProvenance>,
}

/// A composition that cannot be performed.
///
/// Every variant names the pointer and the pieces involved, because a merge
/// failure a user cannot locate is a merge failure they cannot fix.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum MergeError {
    NotAnObject {
        piece: usize,
    },
    TypeConflict {
        pointer: Pointer,
        earlier: usize,
        later: usize,
        earlier_type: &'static str,
        later_type: &'static str,
    },
    StrategyTargetMissing {
        pointer: Pointer,
    },
    StrategyTargetNotAnArray {
        pointer: Pointer,
        actual_type: &'static str,
    },
    DuplicateMergeKey {
        pointer: Pointer,
        key: String,
        value: String,
    },
    MergeKeyMissing {
        pointer: Pointer,
        key: String,
        piece: usize,
    },
}

/// Names a JSON node's type for a diagnostic.
const fn type_name(value: &Value) -> &'static str {
    match *value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

/// Folds the pieces left to right under the declared strategies.
pub(crate) fn compose(
    pieces: &[Value],
    strategies: &StrategyTable,
) -> Result<Composed, MergeError> {
    for (index, piece) in pieces.iter().enumerate() {
        if !piece.is_object() {
            return Err(MergeError::NotAnObject { piece: index });
        }
    }
    let mut state = Fold {
        document: Value::Object(serde_json::Map::new()),
        keys: BTreeMap::new(),
        containers: BTreeMap::new(),
        strategies,
    };
    for (index, piece) in pieces.iter().enumerate() {
        state.absorb(&mut Vec::new(), index, piece)?;
    }
    state.verify_every_strategy_applied()?;
    Ok(Composed {
        document: canonicalize(state.document),
        keys: state.keys,
    })
}

/// Rebuilds every object with its keys in sorted order.
///
/// Explicit rather than inherited. `serde_json::Map` is a `BTreeMap` only while
/// the `preserve_order` feature is off, and this fold's whole purpose is to be
/// nameable by its inputs — a document whose key order depended on a feature
/// flag somewhere in the dependency graph could not be.
fn canonicalize(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut sorted: Vec<(String, Value)> = map.into_iter().collect();
            sorted.sort_by(|left, right| left.0.cmp(&right.0));
            Value::Object(
                sorted
                    .into_iter()
                    .map(|(key, nested)| (key, canonicalize(nested)))
                    .collect(),
            )
        }
        Value::Array(items) => Value::Array(items.into_iter().map(canonicalize).collect()),
        scalar => scalar,
    }
}

struct Fold<'a> {
    document: Value,
    keys: BTreeMap<Pointer, KeyProvenance>,
    /// Which piece first created each object container.
    ///
    /// Not provenance and never emitted: a container is not an entry. It exists
    /// so a type conflict against a container can still name the piece that
    /// made it one, which is the whole point of reporting the conflict.
    containers: BTreeMap<Pointer, usize>,
    strategies: &'a StrategyTable,
}

impl Fold<'_> {
    /// Merges one piece's subtree into the document at the current path.
    fn absorb(
        &mut self,
        path: &mut Vec<String>,
        piece: usize,
        source: &Value,
    ) -> Result<(), MergeError> {
        let Value::Object(members) = source else {
            unreachable!("absorb is only entered with an object");
        };
        for (key, value) in members {
            path.push(key.clone());
            let result = self.absorb_member(path, piece, value);
            path.pop();
            result?;
        }
        Ok(())
    }

    /// Merges one key's value, choosing between recursion and a leaf write.
    fn absorb_member(
        &mut self,
        path: &mut Vec<String>,
        piece: usize,
        value: &Value,
    ) -> Result<(), MergeError> {
        let pointer = Pointer::from_tokens(path);
        let existing = at(&self.document, path);
        match (existing, value) {
            // Two objects: recurse. Containers carry no provenance of their own.
            (Some(Value::Object(_)), Value::Object(_)) => self.absorb(path, piece, value),
            // A key nobody has written yet.
            (None, Value::Object(_)) => {
                put(
                    &mut self.document,
                    path,
                    Value::Object(serde_json::Map::new()),
                );
                self.containers.insert(pointer, piece);
                self.absorb(path, piece, value)
            }
            (None, leaf) => {
                put(&mut self.document, path, leaf.clone());
                self.keys.insert(pointer, KeyProvenance::single(piece));
                Ok(())
            }
            (Some(previous), incoming) => {
                let (previous_type, incoming_type) = (type_name(previous), type_name(incoming));
                // Null is a value like any other, so the only thing that makes
                // two nodes incompatible is one of them being a container the
                // other is not. Anything else is an ordinary override.
                if (previous_type == "object") != (incoming_type == "object") {
                    let earlier = self.keys.get(&pointer).map_or_else(
                        || self.containers.get(&pointer).copied().unwrap_or(piece),
                        |provenance| provenance.piece,
                    );
                    return Err(MergeError::TypeConflict {
                        pointer,
                        earlier,
                        later: piece,
                        earlier_type: previous_type,
                        later_type: incoming_type,
                    });
                }
                if previous_type == "array"
                    && incoming_type == "array"
                    && let Some(strategy) = self.strategies.get(&pointer)
                {
                    return self.apply_strategy(path, &pointer, piece, incoming, strategy);
                }
                self.override_leaf(path, &pointer, piece, incoming);
                Ok(())
            }
        }
    }

    /// Replaces a leaf and records the chain it displaced.
    fn override_leaf(&mut self, path: &[String], pointer: &Pointer, piece: usize, value: &Value) {
        put(&mut self.document, path, value.clone());
        let entry = self
            .keys
            .entry(pointer.clone())
            .or_insert_with(|| KeyProvenance::single(piece));
        // The previous winner joins the overridden chain, and a merged array
        // that is now being replaced outright loses its contributor account
        // along with its elements.
        if entry.piece != piece {
            entry.overrode.push(entry.piece);
        }
        entry.piece = piece;
        entry.strategy = None;
        entry.contributors.clear();
    }

    /// Applies `concat` or `merge-by-key` to two arrays.
    fn apply_strategy(
        &mut self,
        path: &[String],
        pointer: &Pointer,
        piece: usize,
        incoming: &Value,
        strategy: &ArrayStrategy,
    ) -> Result<(), MergeError> {
        let existing = match at(&self.document, path) {
            Some(Value::Array(items)) => items.clone(),
            _ => Vec::new(),
        };
        let addition = incoming.as_array().cloned().unwrap_or_default();
        let merged = match *strategy {
            ArrayStrategy::Concat => {
                let mut merged = existing;
                merged.extend(addition);
                merged
            }
            ArrayStrategy::MergeByKey { ref key } => {
                merge_by_key(pointer, key, existing, addition, piece)?
            }
        };
        put(&mut self.document, path, Value::Array(merged));
        let entry = self
            .keys
            .entry(pointer.clone())
            .or_insert_with(|| KeyProvenance::single(piece));
        let mut contributors = std::mem::take(&mut entry.contributors);
        if contributors.is_empty() {
            contributors.push(entry.piece);
        }
        if contributors.last() != Some(&piece) {
            contributors.push(piece);
        }
        entry.piece = piece;
        entry.contributors = contributors;
        entry.strategy = Some(strategy.clone());
        entry.overrode.clear();
        Ok(())
    }

    /// Confirms every declared pointer addressed an array that exists.
    ///
    /// An error rather than a warning: a strategy that silently applies to
    /// nothing is the same silent-wrong-settings bug the unknown-key rule
    /// exists to prevent (`configuration.md#declaring-a-strategy`).
    fn verify_every_strategy_applied(&self) -> Result<(), MergeError> {
        for (pointer, _) in self.strategies.iter() {
            let tokens = pointer.tokens();
            match at(&self.document, &tokens) {
                None => {
                    return Err(MergeError::StrategyTargetMissing {
                        pointer: pointer.clone(),
                    });
                }
                Some(Value::Array(_)) => {}
                Some(other) => {
                    return Err(MergeError::StrategyTargetNotAnArray {
                        pointer: pointer.clone(),
                        actual_type: type_name(other),
                    });
                }
            }
        }
        Ok(())
    }
}

/// Matches elements on a named field, merging matches and appending the rest.
fn merge_by_key(
    pointer: &Pointer,
    key: &str,
    existing: Vec<Value>,
    addition: Vec<Value>,
    piece: usize,
) -> Result<Vec<Value>, MergeError> {
    let identity = |element: &Value, piece: usize| -> Result<String, MergeError> {
        element
            .get(key)
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| MergeError::MergeKeyMissing {
                pointer: pointer.clone(),
                key: key.to_owned(),
                piece,
            })
    };
    let mut merged: Vec<Value> = Vec::with_capacity(existing.len() + addition.len());
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    for element in existing {
        let value = identity(&element, piece)?;
        if seen.insert(value.clone(), merged.len()).is_some() {
            return Err(MergeError::DuplicateMergeKey {
                pointer: pointer.clone(),
                key: key.to_owned(),
                value,
            });
        }
        merged.push(element);
    }
    let mut added: BTreeSet<String> = BTreeSet::new();
    for element in addition {
        let value = identity(&element, piece)?;
        if !added.insert(value.clone()) {
            return Err(MergeError::DuplicateMergeKey {
                pointer: pointer.clone(),
                key: key.to_owned(),
                value,
            });
        }
        if let Some(&index) = seen.get(&value) {
            shallow_merge(&mut merged[index], &element);
        } else {
            seen.insert(value, merged.len());
            merged.push(element);
        }
    }
    Ok(merged)
}

/// Merges one matched element into another, recursively over objects.
///
/// Element-level and provenance-free: the array is one leaf, so what happens
/// inside a matched pair is part of that leaf's value rather than a second
/// level of contributor accounting.
fn shallow_merge(into: &mut Value, from: &Value) {
    match (into, from) {
        (Value::Object(target), Value::Object(source)) => {
            for (key, value) in source {
                match target.get_mut(key) {
                    Some(existing) => shallow_merge(existing, value),
                    None => {
                        target.insert(key.clone(), value.clone());
                    }
                }
            }
        }
        (target, source) => *target = source.clone(),
    }
}

/// Reads the node at a decoded token path.
fn at<'a>(document: &'a Value, path: &[String]) -> Option<&'a Value> {
    let mut node = document;
    for token in path {
        node = node.get(token)?;
    }
    Some(node)
}

/// Writes a node at a decoded token path, creating intermediate objects.
///
/// Every ancestor of a leaf is an object by construction: `absorb_member`
/// creates one before it descends, and a type conflict is refused before it can
/// leave a non-object in the way. A missing ancestor is therefore a bug in this
/// module rather than bad input, and it is dropped rather than panicking on a
/// user's launch.
fn put(document: &mut Value, path: &[String], value: Value) {
    let Some((last, parents)) = path.split_last() else {
        *document = value;
        return;
    };
    let mut node = document;
    for token in parents {
        let Some(members) = node.as_object_mut() else {
            return;
        };
        node = members
            .entry(token.clone())
            .or_insert_with(|| Value::Object(serde_json::Map::new()));
    }
    if let Some(members) = node.as_object_mut() {
        members.insert(last.clone(), value);
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    fn value(text: &str) -> Value {
        serde_json::from_str(text).expect("json")
    }

    fn table(yaml: &str) -> StrategyTable {
        serde_yaml_ng::from_str(yaml).expect("strategy table")
    }

    fn pointer(text: &str) -> Pointer {
        Pointer::parse(text).expect("valid pointer")
    }

    fn fold(pieces: &[&str], strategies: &StrategyTable) -> Composed {
        let parsed: Vec<Value> = pieces.iter().map(|text| value(text)).collect();
        compose(&parsed, strategies).expect("composes")
    }

    #[test]
    fn objects_merge_key_by_key_recursively() {
        let composed = fold(
            &[
                r#"{"env":{"A":"1","B":"2"}}"#,
                r#"{"env":{"B":"3","C":"4"}}"#,
            ],
            &StrategyTable::default(),
        );
        assert_eq!(
            composed.document,
            value(r#"{"env":{"A":"1","B":"3","C":"4"}}"#)
        );
    }

    #[test]
    fn a_scalar_takes_its_last_writer() {
        let composed = fold(
            &[r#"{"model":"a"}"#, r#"{"model":"b"}"#],
            &StrategyTable::default(),
        );
        assert_eq!(composed.document["model"], "b");
        assert_eq!(composed.keys[&pointer("/model")].piece, 1);
    }

    #[test]
    fn a_scalar_records_the_pieces_it_overrode_in_order() {
        let composed = fold(
            &[r#"{"model":"a"}"#, r#"{"model":"b"}"#, r#"{"model":"c"}"#],
            &StrategyTable::default(),
        );
        let provenance = &composed.keys[&pointer("/model")];
        assert_eq!(provenance.piece, 2);
        assert_eq!(provenance.overrode, [0, 1]);
    }

    #[test]
    fn an_unlisted_array_is_replaced() {
        let composed = fold(
            &[r#"{"allow":["a","b"]}"#, r#"{"allow":["c"]}"#],
            &StrategyTable::default(),
        );
        assert_eq!(composed.document["allow"], value(r#"["c"]"#));
        assert!(composed.keys[&pointer("/allow")].strategy.is_none());
    }

    #[test]
    fn a_concat_array_appends_in_layer_order() {
        let composed = fold(
            &[
                r#"{"allow":["a"]}"#,
                r#"{"allow":["b"]}"#,
                r#"{"allow":["c"]}"#,
            ],
            &table("\"/allow\":\n  strategy: concat\n"),
        );
        assert_eq!(composed.document["allow"], value(r#"["a","b","c"]"#));
    }

    #[test]
    fn a_concat_array_records_every_contributor() {
        let composed = fold(
            &[
                r#"{"allow":["a"]}"#,
                r#"{"allow":["b"]}"#,
                r#"{"allow":["c"]}"#,
            ],
            &table("\"/allow\":\n  strategy: concat\n"),
        );
        let provenance = &composed.keys[&pointer("/allow")];
        assert_eq!(provenance.contributors, [0, 1, 2]);
        assert_eq!(provenance.piece, 2);
        assert_eq!(provenance.strategy, Some(ArrayStrategy::Concat));
        assert!(provenance.overrode.is_empty());
    }

    #[test]
    fn merge_by_key_merges_matched_elements_recursively() {
        let composed = fold(
            &[
                r#"{"hooks":[{"matcher":"Bash","run":"a","opts":{"x":1}}]}"#,
                r#"{"hooks":[{"matcher":"Bash","opts":{"y":2}}]}"#,
            ],
            &table("\"/hooks\":\n  strategy: merge-by-key\n  key: matcher\n"),
        );
        assert_eq!(
            composed.document["hooks"],
            value(r#"[{"matcher":"Bash","run":"a","opts":{"x":1,"y":2}}]"#)
        );
    }

    #[test]
    fn merge_by_key_appends_unmatched_elements_in_layer_order() {
        let composed = fold(
            &[
                r#"{"hooks":[{"matcher":"Bash"}]}"#,
                r#"{"hooks":[{"matcher":"Edit"}]}"#,
            ],
            &table("\"/hooks\":\n  strategy: merge-by-key\n  key: matcher\n"),
        );
        let hooks = composed.document["hooks"].as_array().expect("array");
        assert_eq!(hooks[0]["matcher"], "Bash");
        assert_eq!(hooks[1]["matcher"], "Edit");
    }

    #[test]
    fn merge_by_key_rejects_a_duplicate_key_value() {
        let pieces = [
            value(r#"{"hooks":[{"matcher":"Bash"}]}"#),
            value(r#"{"hooks":[{"matcher":"Edit"},{"matcher":"Edit"}]}"#),
        ];
        let error = compose(
            &pieces,
            &table("\"/hooks\":\n  strategy: merge-by-key\n  key: matcher\n"),
        )
        .expect_err("a duplicate key value is ambiguous");
        assert!(matches!(error, MergeError::DuplicateMergeKey { .. }));
    }

    #[test]
    fn merge_by_key_rejects_an_element_without_the_key() {
        let pieces = [
            value(r#"{"hooks":[{"matcher":"Bash"}]}"#),
            value(r#"{"hooks":[{"run":"x"}]}"#),
        ];
        let error = compose(
            &pieces,
            &table("\"/hooks\":\n  strategy: merge-by-key\n  key: matcher\n"),
        )
        .expect_err("an element without the key cannot be matched");
        assert!(matches!(error, MergeError::MergeKeyMissing { .. }));
    }

    #[test]
    fn a_type_conflict_names_the_pointer_and_both_pieces() {
        let pieces = [value(r#"{"env":{"A":"1"}}"#), value(r#"{"env":"nope"}"#)];
        let error = compose(&pieces, &StrategyTable::default()).expect_err("object versus string");
        match error {
            MergeError::TypeConflict {
                pointer: at,
                earlier,
                later,
                earlier_type,
                later_type,
            } => {
                assert_eq!(at.as_str(), "/env");
                assert_eq!((earlier, later), (0, 1));
                assert_eq!((earlier_type, later_type), ("object", "string"));
            }
            other => panic!("expected a type conflict, got {other:?}"),
        }
    }

    #[test]
    fn a_strategy_pointer_matching_no_key_is_rejected() {
        let pieces = [value(r#"{"allow":["a"]}"#)];
        let error = compose(&pieces, &table("\"/deny\":\n  strategy: concat\n"))
            .expect_err("the pointer addresses nothing");
        assert!(matches!(error, MergeError::StrategyTargetMissing { .. }));
    }

    #[test]
    fn a_strategy_pointer_naming_a_non_array_is_rejected() {
        let pieces = [value(r#"{"model":"a"}"#)];
        let error = compose(&pieces, &table("\"/model\":\n  strategy: concat\n"))
            .expect_err("the pointer addresses a scalar");
        assert!(matches!(
            error,
            MergeError::StrategyTargetNotAnArray {
                actual_type: "string",
                ..
            }
        ));
    }

    #[test]
    fn an_escaped_pointer_addresses_a_key_containing_a_slash() {
        let composed = fold(
            &[r#"{"a/b":["x"]}"#, r#"{"a/b":["y"]}"#],
            &table("\"/a~1b\":\n  strategy: concat\n"),
        );
        assert_eq!(composed.document["a/b"], value(r#"["x","y"]"#));
        assert_eq!(
            composed.keys[&pointer("/a~1b")].strategy,
            Some(ArrayStrategy::Concat)
        );
    }

    #[test]
    fn the_same_pieces_compose_byte_identical_output() {
        let pieces = [r#"{"b":1,"a":2}"#, r#"{"c":3}"#];
        let first = fold(&pieces, &StrategyTable::default());
        let second = fold(&pieces, &StrategyTable::default());
        assert_eq!(
            serde_json::to_string(&first.document).expect("json"),
            serde_json::to_string(&second.document).expect("json")
        );
    }

    /// Determinism has to survive the input's own key order, or the entry key
    /// would name bytes that depend on how a piece happened to be typed.
    #[test]
    fn differing_source_key_order_composes_identical_bytes() {
        let one = fold(&[r#"{"b":1,"a":2,"c":3}"#], &StrategyTable::default());
        let other = fold(&[r#"{"c":3,"a":2,"b":1}"#], &StrategyTable::default());
        assert_eq!(
            serde_json::to_string(&one.document).expect("json"),
            serde_json::to_string(&other.document).expect("json")
        );
        assert_eq!(
            serde_json::to_string(&one.document).expect("json"),
            r#"{"a":2,"b":1,"c":3}"#
        );
    }

    #[test]
    fn a_single_piece_key_records_one_contributor_and_no_chain() {
        let composed = fold(&[r#"{"model":"a"}"#], &StrategyTable::default());
        let provenance = &composed.keys[&pointer("/model")];
        assert_eq!(provenance.piece, 0);
        assert!(provenance.overrode.is_empty());
        assert!(provenance.contributors.is_empty());
        assert!(provenance.strategy.is_none());
    }

    #[test]
    fn a_piece_that_is_not_an_object_is_rejected() {
        let error = compose(&[value("[1,2]")], &StrategyTable::default())
            .expect_err("arrays are not pieces");
        assert_eq!(error, MergeError::NotAnObject { piece: 0 });
    }

    /// Containers are not entries: `/env` explains nothing `/env/A` does not.
    #[test]
    fn provenance_addresses_leaves_only() {
        let composed = fold(&[r#"{"env":{"A":"1"}}"#], &StrategyTable::default());
        let addressed: Vec<&str> = composed.keys.keys().map(Pointer::as_str).collect();
        assert_eq!(addressed, ["/env/A"]);
    }
}
