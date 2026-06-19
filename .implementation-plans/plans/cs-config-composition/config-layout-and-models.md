# Config Composition R1: XDG Layout, Manifest & Piece Models

> Plan: cs-config-composition | Round: 1 of 4 | Complexity: L | Executor: prex (EF 1.5) | Generated: 2026-06-19 |
> Repo: /workspaces/claude-session

## Context

`claude-session`'s config source of truth is user-editable XDG config, not native `claude` files.
Profiles are YAML manifests that compose partial `settings.json` JSON pieces. This round establishes
the on-disk config layout under `~/.config/claude-session/`, the manifest and piece models, and their
loader validation — the inputs the merge engine (round 2) consumes. `cs-foundation` provides the
figment `Config`, error layers, and `Ui`; the blessed `serde_yaml_ng` dep is added here for manifests.

## Previous Rounds

`cs-foundation`: crate, figment `Config` (`deny_unknown_fields`, XDG paths), error/ui. `cs-isolation`:
the session dir the output later lands in. Expect these to exist.

## Scope of This Round

- IN scope: define the layout `~/.config/claude-session/` with `manifests/<profile>.yaml` and
  `settings/<piece>.json`; `services/config_compose/manifest.rs` (parse a manifest with `serde_yaml_ng`:
  ordered `layers: [string]`, `deny_unknown_fields`, require `minItems:1`); `services/config_compose/
  piece.rs` (load JSON pieces as `serde_json::Value`, preserving file path + layer name for error
  reporting); resolve a profile name → manifest path → ordered piece paths, erroring clearly when a
  referenced piece is missing.
- OUT of scope: the merge engine (round 2), generation/output (round 3), CLI verbs (round 4).

## Current State

### Key Files

- `/workspaces/claude-session/src/services.rs` (+ `src/services/`) — add `config_compose/` submodule
  with `manifest.rs`, `piece.rs`.
- `/workspaces/claude-session/src/config.rs` (+ `src/config/`) — knows `~/.config/claude-session`.
- `/workspaces/claude-session/Cargo.toml` — add `serde_yaml_ng` for manifests via
  `cargo add serde_yaml_ng` (never hand-edit `[dependencies]`).

### Existing Patterns

Reference model (devcontainerctl, inspiration only): one YAML manifest per profile with `layers:
[base, agents, python]` (ordered; last wins); each layer a partial JSON piece referenced by name;
schema `additionalProperties: false`, `minItems: 1`. Error reporting names the failing file ("Layer
'X' referenced in manifest 'Y' not found: <path>"). codex-session's `manifest.rs`/`layer.rs` show the
Rust parse shape. Use the blessed `serde_yaml_ng` (NOT `serde_yaml`).

## Implementation Steps

### First Step: Mark this round as started

In this plan's `queue-rounds.yaml`, set this round's (`item: config-layout-and-models`) `status` to
`doing`.

### Step 1: Layout

Define and document `~/.config/claude-session/{manifests/<profile>.yaml, settings/<piece>.json}` in
`config/`.

### Step 2: Manifest model

In `services/config_compose/manifest.rs`, parse the manifest (`serde_yaml_ng`, `deny_unknown_fields`,
ordered non-empty `layers`).

### Step 3: Piece loader + resolution

In `services/config_compose/piece.rs`, load JSON pieces as `serde_json::Value` keeping path + layer
name; resolve profile → manifest → ordered piece paths; error clearly on a missing piece.

### Step 4: Tests

Unit-test manifest parsing (reject unknown fields / empty layers) and piece resolution (missing piece
names the file) with fixtures.

### Final Step: Update the queue

1. In this plan's `queue-rounds.yaml`, set this round's (`item: config-layout-and-models`) `status` to
   `done`.

## Acceptance Criteria

- [ ] A YAML manifest with ordered `layers` parses; unknown fields / empty layers are rejected with a
      clear error.
- [ ] JSON pieces load as `serde_json::Value` retaining path + layer name for diagnostics.
- [ ] A missing referenced piece produces an error naming the manifest and the path.
- [ ] Tests pass with fixtures.
- [ ] This plan's `queue-rounds.yaml` shows round `config-layout-and-models` as `done`.

## Next Round

Round 2 (`merge-engine`) deep-merges the ordered pieces with configurable per-key strategies and
per-key provenance.
