# Config Composition R2: serde_json Merge Engine, Strategies & Provenance

> Plan: cs-config-composition | Round: 2 of 4 | Complexity: L | Executor: prex (EF 1.5) | Generated: 2026-06-19 | Repo: repository root

## Context

The heart of config composition is a deterministic merge of the ordered JSON pieces into one final `settings.json` value. It must be idiomatic Rust (`serde_json::Value`, not jq), support configurable per-key array strategies, and record per-key provenance (which piece set each key) so errors and the `config` verbs can explain the result. This round builds that engine. Round 1 produced the profile + piece models and ordered resolution.

## Previous Rounds

This plan round 1: `profile.rs`, `piece.rs`, profile→ordered-pieces resolution. Expect those to exist.

## Scope of This Round

- IN scope: `services/config_compose/merge.rs` — a recursive `serde_json::Value` deep merge folding the ordered pieces left-to-right; **scalars last-wins**; **objects merge-by-key**; **arrays configurable per-key** (default `replace`; opt-in `concat` and `merge-by-key`), with the strategy table sourced from an optional policy in the profile; **per-key provenance** (track which piece set each leaf key, e.g. a parallel provenance map keyed by JSON path); typed `MergeError` (e.g. type-conflict at a path) naming the offending piece + path.
- OUT of scope: writing the output file / schema validation / freshness (round 3); CLI verbs (round 4).

## Current State

### Key Files

- `src/services/config_compose/merge.rs` — new.
- `src/services/config_compose/profile.rs` — may carry an optional per-key array-strategy policy.
- `src/error.rs` — add `MergeError` mapping.

### Existing Patterns

The merge semantics are specified in `docs/reference/configuration.md` and recorded in `docs/decisions/ADR-0010-compose-native-settings-from-declared-layers.md`. Implement that table exactly. For orientation, the shape being replaced is a shell-and-`jq` pipeline of roughly this form:

```text
$base * $tmpl                                   # scalars: last-wins (recursive object merge)
.mounts = (($base.mounts) + ($tmpl.mounts))     # arrays: CONCATENATE (a per-key strategy)
.containerEnv = (($base.containerEnv) * ($tmpl.containerEnv))  # objects: merge-by-key
```

Documented rule: "mounts arrays are concatenated; containerEnv/postCreateCommand/remoteEnv objects are merged by key; scalar fields use last-wins." Improve on it: default array strategy = `replace`, opt-in `concat`/`merge-by-key` per key; record provenance per key.

Two things the specification fixes that this round predates. The strategy table is the profile's optional `array_strategies` map, **keyed by RFC 6901 JSON Pointer** rather than a dotted path, because the child renders nested settings keys dotted and a dotted strategy key stops being addressable the moment a settings key contains a dot. A non-canonical pointer, one naming a non-array, one matching no key, and duplicate merge-key values are each `DataFormat` — a strategy that silently applies to nothing is the bug the unknown-key rule exists to prevent. And provenance is **not winner-only**: where more than one piece touched a key the record carries the ordered chain, `overrode` for a scalar and `contributors` plus `strategy` for a merged array, because "the setting is wrong because this piece overrode that one" is the answer the sidecar exists to give. `docs/reference/configuration.md` §§ Declaring a strategy and Provenance sidecar own both.

## Implementation Steps

### First Step: Mark this round as started

In this plan's `queue-rounds.yaml`, set this round's (`item: merge-engine`) `status` to `doing`.

### Step 1: Deep merge

In `services/config_compose/merge.rs`, implement the recursive `serde_json::Value` fold: objects merge-by-key, scalars last-wins, arrays per the configured strategy.

### Step 2: Per-key strategies

Add the strategy table (default `replace`; opt-in `concat`/`merge-by-key`) from the profile's optional `array_strategies`, keyed by RFC 6901 pointer. `replace` is never written — it is what an unlisted array does.

### Step 3: Provenance

Track per-key provenance (which piece set each leaf), returned alongside the merged value.

### Step 4: Errors + tests

Add `MergeError` (type-conflict at a path, naming the piece). Unit-test scalar last-wins, object merge-by-key, each array strategy, and provenance, with fixtures.

### Final Step: Update the queue

1. In this plan's `queue-rounds.yaml`, set this round's (`item: merge-engine`) `status` to `done`.

## Acceptance Criteria

- [ ] The merge folds ordered pieces: scalars last-wins, objects merge-by-key, arrays per the configured strategy (default `replace`, opt-in `concat`/`merge-by-key`).
- [ ] Per-key provenance attributes each leaf key to the winning piece, and carries the ordered chain wherever more than one piece touched it.
- [ ] A type-conflict surfaces a `MergeError` naming the piece + path.
- [ ] A non-canonical pointer, a pointer naming a non-array, a pointer matching no key, and duplicate merge-key values each surface as `DataFormat`.
- [ ] Unit tests cover all strategies + provenance, including the chain and each pointer error.
- [ ] This plan's `queue-rounds.yaml` shows round `merge-engine` as `done`.

## Next Round

Round 3 (`generate-settings`) writes the merged value as the native `settings.json` atomically into the composed-settings store under its input-digest name, validates it, writes a provenance sidecar, and supplies the file to the child through the `--settings` prefix.
