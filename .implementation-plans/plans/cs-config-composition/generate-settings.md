# Config Composition R3: Generate the Native settings.json

> Plan: cs-config-composition | Round: 3 of 4 | Complexity: L | Executor: prex (EF 1.5) | Generated: 2026-06-19 | Repo: repository root

## Context

The merged config value must become the native `settings.json` the child is handed for the group — written atomically, validated, regenerated only when stale, with a provenance sidecar. This round wires the merge engine's output to disk. Round 1 built models, round 2 built the merge engine; `cs-isolation` provides the resolved group directory.

## Previous Rounds

This plan R1: manifest/piece models + resolution. R2: deep-merge engine + provenance. `cs-isolation`: `AppContext`-resolved per-account/per-group session dir. Expect all to exist.

## Scope of This Round

- IN scope: `services/config_compose/generate.rs` — compose (resolve profile → pieces → merge) and write the final native `settings.json` atomically (tempfile → rename) into the resolved group directory; **freshness** check inspecting every referenced piece's mtime (regenerate only if any piece or the manifest is newer than the output); a provenance sidecar `.claude-session-compose.json` (manifest, ordered layers + paths, per-key provenance); **schema validation** of the merged `settings.json` (pragmatic — validate the wrapper-owned structure; allow unknown native keys unless a maintained schema is supplied); hook composition into the pass-through path so the group has a fresh `settings.json` before the child runs, and supply its absolute path into the wrapper-owned argv prefix `cs-wrapper-runtime` built, per `docs/decisions/ADR-0028-pass-composed-settings-with-the-native-flag.md`.
- OUT of scope: the `config`/`profile` CLI verbs (round 4).

## Current State

### Key Files

- `src/services/config_compose/generate.rs` — new.
- `src/services/session/dir.rs` — provides the group directory to write into.
- `src/commands/pass_through.rs` — invoke compose before spawn (alongside the auth gate from `cs-accounts-auth` when present).

### Existing Patterns

Generation, freshness, the provenance sidecar, and the validation posture are specified in `docs/reference/configuration.md`; the output paths and modes are in `docs/reference/xdg-storage.md`.

Three rules carry the weight. The write is **atomic** — a temporary file in the same directory, then rename — so a reader never sees a truncated settings file. The merge is **deterministic**, producing byte-identical output from identical inputs, or the freshness check and diffs are both useless. And **freshness compares against the manifest and every referenced piece**, not just the manifest: editing a piece without touching the manifest otherwise leaves stale settings in place, and the symptom — an edit that appears to do nothing — is genuinely hard to diagnose.

Validation is pragmatic: validate the structure the wrapper owns and the well-formedness of the whole, but do **not** reject unknown keys in the child's schema. That schema is externally owned and evolves; it is tracked in `docs/reference/research-tracking.yaml`.

The wrapper writes the generated settings document and its sidecar, and nothing else. Project trust, history, and onboarding are child-owned state inside the account's `config/`; the wrapper does not seed, copy, or sync them back — `docs/decisions/ADR-0025-share-one-native-login-per-account.md` removed that obligation, and `docs/decisions/ADR-0004-spawn-and-wait-child-supervision.md` is amended accordingly.

## Implementation Steps

### First Step: Mark this round as started

In this plan's `queue-rounds.yaml`, set this round's (`item: generate-settings`) `status` to `doing`.

### Step 1: Compose + atomic write

In `services/config_compose/generate.rs`, resolve → merge → write `settings.json` atomically into the group directory.

### Step 2: Freshness + sidecar

Add the freshness check (every piece + manifest mtime) and write the `.claude-session-compose.json` provenance sidecar.

### Step 3: Validation

Validate the merged `settings.json` pragmatically: check the structure the wrapper owns and the well-formedness of the whole, and accept unknown native keys.

### Step 4: Hook into pass-through + tests

Invoke compose before spawn in `pass_through.rs` and supply the generated file's absolute path as the `--settings` prefix, leaving every user token an untouched suffix. Integration-test that a profile produces the expected `settings.json` in the group directory and that an unchanged profile is not regenerated.

### Final Step: Update the queue

1. In this plan's `queue-rounds.yaml`, set this round's (`item: generate-settings`) `status` to `done`.

## Acceptance Criteria

- [ ] Composing a profile writes the native `settings.json` atomically into the resolved group directory.
- [ ] Freshness checks every referenced piece + the manifest; an unchanged profile is not regenerated.
- [ ] A `.claude-session-compose.json` provenance sidecar is written; the merged settings pass validation.
- [ ] The generated file reaches the child as a `--settings` prefix, with the user's argv preserved as an untouched suffix.
- [ ] Integration tests pass.
- [ ] This plan's `queue-rounds.yaml` shows round `generate-settings` as `done`.

## Next Round

Round 4 (`config-commands`) exposes every `config` and `profile` subcommand in the table in `docs/reference/configuration.md`, surfacing provenance and unknown-key warnings.
