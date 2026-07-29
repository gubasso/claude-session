# Config Composition R3: Generate native settings.json + Trust State

> Plan: cs-config-composition | Round: 3 of 4 | Complexity: L | Executor: prex (EF 1.5) | Generated: 2026-06-19 | Repo: repository root

## Context

The merged config value must become the native `settings.json` that the real `claude` reads inside the isolated session — written atomically, validated, regenerated only when stale, with a provenance sidecar, plus conservative handling of `.claude.json` trust/onboarding state. This round wires the merge engine's output to disk. Round 1 built models, round 2 built the merge engine; `cs-isolation` provides the resolved session dir.

## Previous Rounds

This plan R1: manifest/piece models + resolution. R2: deep-merge engine + provenance. `cs-isolation`: `AppContext`-resolved per-account/per-group session dir. Expect all to exist.

## Scope of This Round

- IN scope: `services/config_compose/generate.rs` — compose (resolve profile → pieces → merge) and write the final native `settings.json` atomically (tempfile → rename) into the resolved session dir; **freshness** check inspecting every referenced piece's mtime (regenerate only if any piece or the manifest is newer than the output); a provenance sidecar `.claude-session-compose.json` (manifest, ordered layers + paths, per-key provenance); **schema validation** of the merged `settings.json` (pragmatic — validate the wrapper-owned structure; allow unknown native keys unless a maintained schema is supplied); conservative `.claude.json` trust/onboarding seeding/merge (do not clobber newer state) under a lock; hook composition into the pass-through path so the isolated session gets a fresh `settings.json` before `claude` runs.
- OUT of scope: the `config`/`profile` CLI verbs (round 4).

## Current State

### Key Files

- `src/services/config_compose/generate.rs` — new.
- `src/services/session/dir.rs` — provides the session dir to write into.
- `src/commands/pass_through.rs` — invoke compose before spawn (alongside the auth gate from `cs-accounts-auth` when present).

### Existing Patterns

Generation, freshness, the provenance sidecar, and the validation posture are specified in `docs/reference/configuration.md`; the output paths and modes are in `docs/reference/xdg-storage.md`.

Three rules carry the weight. The write is **atomic** — a temporary file in the same directory, then rename — so a reader never sees a truncated settings file. The merge is **deterministic**, producing byte-identical output from identical inputs, or the freshness check and diffs are both useless. And **freshness compares against the manifest and every referenced piece**, not just the manifest: editing a piece without touching the manifest otherwise leaves stale settings in place, and the symptom — an edit that appears to do nothing — is genuinely hard to diagnose.

Validation is pragmatic: validate the structure the wrapper owns and the well-formedness of the whole, but do **not** reject unknown keys in the child's schema. That schema is externally owned and evolves; it is tracked in `docs/reference/research-tracking.yaml`.

Project-trust state lives in a file separate from the settings file — itself a perishable fact — and is seeded in and synced back under a lock, merging conservatively.

## Implementation Steps

### First Step: Mark this round as started

In this plan's `queue-rounds.yaml`, set this round's (`item: generate-settings`) `status` to `doing`.

### Step 1: Compose + atomic write

In `services/config_compose/generate.rs`, resolve → merge → write `settings.json` atomically into the session dir.

### Step 2: Freshness + sidecar

Add the freshness check (every piece + manifest mtime) and write the `.claude-session-compose.json` provenance sidecar.

### Step 3: Schema validation + trust state

Validate the merged `settings.json` pragmatically; conservatively seed/merge `.claude.json` trust state under a lock without clobbering newer state.

### Step 4: Hook into pass-through + tests

Invoke compose before spawn in `pass_through.rs`. Integration-test that a profile produces the expected `settings.json` in the session dir and that an unchanged profile is not regenerated.

### Final Step: Update the queue

1. In this plan's `queue-rounds.yaml`, set this round's (`item: generate-settings`) `status` to `done`.

## Acceptance Criteria

- [ ] Composing a profile writes the native `settings.json` atomically into the resolved session dir.
- [ ] Freshness checks every referenced piece + the manifest; an unchanged profile is not regenerated.
- [ ] A `.claude-session-compose.json` provenance sidecar is written; the merged settings pass validation.
- [ ] `.claude.json` trust state is seeded/merged conservatively under a lock.
- [ ] Integration tests pass.
- [ ] This plan's `queue-rounds.yaml` shows round `generate-settings` as `done`.

## Next Round

Round 4 (`config-commands`) exposes `config show|path|compose|validate|status` and `profile
list|show`, surfacing provenance and unknown-key warnings.
