# Config Composition R1: XDG Layout, Manifest & Piece Models

> Plan: cs-config-composition | Round: 1 of 4 | Complexity: L | Executor: prex (EF 1.5) | Generated: 2026-06-19 | Repo: repository root

## Context

`claude-session`'s config source of truth is user-editable XDG config, not native `claude` files. Profiles are YAML manifests that compose partial `settings.json` JSON pieces. This round establishes the on-disk config layout under the config base, the manifest and piece models, and their loader validation — the inputs the merge engine (round 2) consumes. `cs-foundation` provides the figment `Config`, error layers, and `Ui`; the blessed `serde_yaml_ng` dep is added here for manifests.

## Previous Rounds

`cs-foundation`: crate, figment `Config` (`deny_unknown_fields`, XDG paths), error/ui. `cs-isolation`: the session dir the output later lands in. Expect these to exist.

## Scope of This Round

- IN scope: implement the config-base layout `manifests/<profile>.yaml` and `settings/<piece>.json` from the artifact table in `docs/reference/xdg-storage.md`; `services/config_compose/manifest.rs` (parse a manifest with `serde_yaml_ng`: ordered `layers: [string]`, `deny_unknown_fields`, require `minItems:1`); `services/config_compose/
  piece.rs` (load JSON pieces as `serde_json::Value`, preserving file path + layer name for error reporting); resolve a profile name → manifest path → ordered piece paths, erroring clearly when a referenced piece is missing.
- OUT of scope: the merge engine (round 2), generation/output (round 3), CLI verbs (round 4).

## Current State

### Key Files

- `src/services.rs` (+ `src/services/`) — add `config_compose/` submodule with `manifest.rs`, `piece.rs`.
- `src/config.rs` (+ `src/config/`) — resolves the config base.
- `Cargo.toml` — add `serde_yaml_ng` for manifests via `cargo add serde_yaml_ng` (never hand-edit `[dependencies]`).

### Existing Patterns

The composition model — read-only JSON pieces plus an ordered YAML manifest per profile — is specified in `docs/reference/configuration.md` and recorded in `docs/decisions/ADR-0010-compose-native-settings-from-declared-layers.md`. Piece and manifest paths come from the artifact table in `docs/reference/xdg-storage.md`; both are user-authored and **read-only to the wrapper**.

A manifest's sole required field is an ordered, non-empty list of piece names. Unknown fields are rejected and an empty list is rejected. A missing referenced piece is an error naming **both** the manifest and the resolved path it looked for — the concrete-value rule from `docs/reference/coding-conventions.md`.

Use `serde_yaml_ng`, not `serde_yaml`, which is deprecated; see the ruled-out list in `docs/reference/dependencies.md`. Add it with `cargo add`.

## Implementation Steps

### First Step: Mark this round as started

In this plan's `queue-rounds.yaml`, set this round's (`item: config-layout-and-models`) `status` to `doing`.

### Step 1: Layout

Implement `{manifests/<profile>.yaml, settings/<piece>.json}` under the resolved config base in `config/`. The base's variable, default, and namespacing come from `docs/reference/xdg-storage.md`; a home-relative path is never hard-coded.

### Step 2: Manifest model

In `services/config_compose/manifest.rs`, parse the manifest (`serde_yaml_ng`, `deny_unknown_fields`, ordered non-empty `layers`).

### Step 3: Piece loader + resolution

In `services/config_compose/piece.rs`, load JSON pieces as `serde_json::Value` keeping path + layer name; resolve profile → manifest → ordered piece paths; error clearly on a missing piece.

### Step 4: Tests

Unit-test manifest parsing (reject unknown fields / empty layers) and piece resolution (missing piece names the file) with fixtures.

### Final Step: Update the queue

1. In this plan's `queue-rounds.yaml`, set this round's (`item: config-layout-and-models`) `status` to `done`.

## Acceptance Criteria

- [ ] A YAML manifest with ordered `layers` parses; unknown fields / empty layers are rejected with a clear error.
- [ ] JSON pieces load as `serde_json::Value` retaining path + layer name for diagnostics.
- [ ] A missing referenced piece produces an error naming the manifest and the path.
- [ ] Tests pass with fixtures.
- [ ] This plan's `queue-rounds.yaml` shows round `config-layout-and-models` as `done`.

## Next Round

Round 2 (`merge-engine`) deep-merges the ordered pieces with configurable per-key strategies and per-key provenance.
