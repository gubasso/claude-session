# Config Composition R2: serde_json Merge Engine, Strategies & Provenance

> Plan: cs-config-composition | Round: 2 of 4 | Complexity: L | Executor: prex (EF 1.5) | Generated: 2026-06-19 | Repo: /workspaces/claude-session

## Context

The heart of config composition is a deterministic merge of the ordered JSON pieces into one final `settings.json` value. It must be idiomatic Rust (`serde_json::Value`, not jq), support configurable per-key array strategies, and record per-key provenance (which piece set each key) so errors and the `config` verbs can explain the result. This round builds that engine. Round 1 produced the manifest + piece models and ordered resolution.

## Previous Rounds

This plan round 1: `manifest.rs`, `piece.rs`, profile→ordered-pieces resolution. Expect those to exist.

## Scope of This Round

- IN scope: `services/config_compose/merge.rs` — a recursive `serde_json::Value` deep merge folding the ordered pieces left-to-right; **scalars last-wins**; **objects merge-by-key**; **arrays configurable per-key** (default `replace`; opt-in `concat` and `merge-by-key`), with the strategy table sourced from an optional policy in the manifest/config; **per-key provenance** (track which piece set each leaf key, e.g. a parallel provenance map keyed by JSON path); typed `MergeError` (e.g. type-conflict at a path) naming the offending piece + path.
- OUT of scope: writing the output file / schema validation / freshness (round 3); CLI verbs (round 4).

## Current State

### Key Files

- `/workspaces/claude-session/src/services/config_compose/merge.rs` — new.
- `/workspaces/claude-session/src/services/config_compose/manifest.rs` — may carry an optional per-key array-strategy policy.
- `/workspaces/claude-session/src/error.rs` — add `MergeError` mapping.

### Existing Patterns

Reference merge semantics (devcontainerctl, inspiration only) — quoted jq, to be reimplemented in Rust:

```text
$base * $tmpl                                   # scalars: last-wins (recursive object merge)
.mounts = (($base.mounts) + ($tmpl.mounts))     # arrays: CONCATENATE (a per-key strategy)
.containerEnv = (($base.containerEnv) * ($tmpl.containerEnv))  # objects: merge-by-key
```

Documented rule: "mounts arrays are concatenated; containerEnv/postCreateCommand/remoteEnv objects are merged by key; scalar fields use last-wins." Improve on it: default array strategy = `replace`, opt-in `concat`/`merge-by-key` per key; record provenance per key.

## Implementation Steps

### First Step: Mark this round as started

In this plan's `queue-rounds.yaml`, set this round's (`item: merge-engine`) `status` to `doing`.

### Step 1: Deep merge

In `services/config_compose/merge.rs`, implement the recursive `serde_json::Value` fold: objects merge-by-key, scalars last-wins, arrays per the configured strategy.

### Step 2: Per-key strategies

Add a strategy table (default `replace`; opt-in `concat`/`merge-by-key`) sourced from optional manifest policy; apply per JSON path/key.

### Step 3: Provenance

Track per-key provenance (which piece set each leaf), returned alongside the merged value.

### Step 4: Errors + tests

Add `MergeError` (type-conflict at a path, naming the piece). Unit-test scalar last-wins, object merge-by-key, each array strategy, and provenance, with fixtures.

### Final Step: Update the queue

1. In this plan's `queue-rounds.yaml`, set this round's (`item: merge-engine`) `status` to `done`.

## Acceptance Criteria

- [ ] The merge folds ordered pieces: scalars last-wins, objects merge-by-key, arrays per the configured strategy (default `replace`, opt-in `concat`/`merge-by-key`).
- [ ] Per-key provenance correctly attributes each leaf key to the piece that set it.
- [ ] A type-conflict surfaces a `MergeError` naming the piece + path.
- [ ] Unit tests cover all strategies + provenance.
- [ ] This plan's `queue-rounds.yaml` shows round `merge-engine` as `done`.

## Next Round

Round 3 (`generate-settings`) writes the merged value as the native `settings.json` atomically into the resolved session dir, validates it, checks freshness, writes a provenance sidecar, and handles `.claude.json` trust/state.
