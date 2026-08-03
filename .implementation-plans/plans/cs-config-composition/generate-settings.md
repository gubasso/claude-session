# Config Composition R3: Generate the Native settings.json

> Plan: cs-config-composition | Round: 3 of 4 | Complexity: L | Executor: prex (EF 1.5) | Generated: 2026-06-19 | Repo: repository root

## Context

The merged config value must become the native `settings.json` the child is handed for the profile — written atomically, validated, written once and never rewritten, with a provenance sidecar. This round wires the merge engine's output to disk. Round 1 built models, round 2 built the merge engine; `cs-isolation` provides the entry key and the store's publication rule.

## Previous Rounds

This plan R1: profile/piece models + resolution. R2: deep-merge engine + provenance. `cs-isolation`: the entry key and the store's publication rule. Expect all to exist.

## Scope of This Round

- IN scope: `services/config_compose/generate.rs` — compose (resolve profile → pieces → merge) and write the final native `settings.json` atomically (tempfile → rename) into the composed-settings store under its input-digest name; **existence** as the whole freshness answer, with an existing entry reused and never rewritten; a provenance sidecar `<entry>.compose.json` (profile, ordered layers + paths, the full input digest, per-key provenance); **schema validation** of the merged `settings.json` (pragmatic — validate the wrapper-owned structure; allow unknown native keys unless a maintained schema is supplied); hook composition into the pass-through path so the entry exists before the child runs, and supply its absolute path into the wrapper-owned argv prefix `cs-wrapper-runtime` built, per `docs/decisions/ADR-0028-pass-composed-settings-with-the-native-flag.md`.
- OUT of scope: the `config`/`profile` CLI verbs (round 4).

## Current State

### Key Files

- `src/services/config_compose/generate.rs` — new.
- `src/services/settings/entry_key.rs` and `store.rs` — provide the entry name and the publication rule.
- `src/commands/pass_through.rs` — invoke compose before spawn (alongside the auth gate from `cs-accounts-auth` when present).

### Existing Patterns

Generation, the provenance sidecar, and the validation posture are specified in `docs/reference/configuration.md`; the entry name, the digest preimage, the write rule, and the modes are in `docs/reference/xdg-storage.md`.

Three rules carry the weight. The write is **atomic** — a temporary file in the same directory, then rename — so a reader never sees a truncated settings file. The merge is **deterministic**, producing byte-identical output from identical inputs, which is what lets an entry be named by its inputs. And **the entry name covers the profile and every referenced piece by content**, so editing a piece names a different entry and stale settings are unrepresentable. Do not add an mtime comparison.

Validation is pragmatic: validate the structure the wrapper owns and the well-formedness of the whole, but do **not** reject unknown keys in the child's schema. That schema is externally owned and evolves; it is tracked in `docs/reference/research-tracking.yaml`.

The wrapper writes the generated settings document and its sidecar, and nothing else. Project trust, history, and onboarding are child-owned state inside the account's `config/`; the wrapper does not seed, copy, or sync them back — `docs/decisions/ADR-0025-share-one-native-login-per-account.md` removed that obligation, and `docs/decisions/ADR-0004-spawn-and-wait-child-supervision.md` is amended accordingly.

## Implementation Steps

### First Step: Mark this round as started

In this plan's `queue-rounds.yaml`, set this round's (`item: generate-settings`) `status` to `doing`.

### Step 1: Compose + atomic write

In `services/config_compose/generate.rs`, resolve → merge → write `settings.json` atomically into the composed-settings store.

### Step 2: Entry key + sidecar

Compute the entry key through `cs-isolation`'s `entry_key`, skip when the entry already exists and its sidecar's recorded digest matches, and write the `<entry>.compose.json` provenance sidecar carrying the full digest. This round is the first that can turn a profile **name** into an entry path, since round 1 supplies the profile and its ordered pieces, so it also extends the `doctor` inspection helper `cs-isolation` left reporting only the store directory.

### Step 3: Validation

Validate the merged `settings.json` pragmatically: check the structure the wrapper owns and the well-formedness of the whole, and accept unknown native keys.

### Step 4: Hook into pass-through + tests

Invoke compose before spawn in `pass_through.rs` and supply the generated file's absolute path as the `--settings` prefix, leaving every user token an untouched suffix. Integration-test that a profile produces the expected `settings.json` at its computed entry path, that an unchanged profile is not recomposed, and that two profiles launched from one terminal produce two entries.

### Final Step: Update the queue

1. In this plan's `queue-rounds.yaml`, set this round's (`item: generate-settings`) `status` to `done`.

## Acceptance Criteria

- [ ] Composing a profile writes the native `settings.json` atomically at its computed entry path.
- [ ] The entry name covers every referenced piece and the profile by content; an unchanged profile is not recomposed and its entry is not rewritten.
- [ ] A `profile-<name>-<digest>.compose.json` provenance sidecar is written beside the entry; the merged settings pass validation.
- [ ] The generated file reaches the child as a `--settings` prefix, with the user's argv preserved as an untouched suffix.
- [ ] Integration tests pass.
- [ ] This plan's `queue-rounds.yaml` shows round `generate-settings` as `done`.

## Next Round

Round 4 (`config-commands`) exposes the `config` and `profile` verbs in the table in `docs/reference/configuration.md`, surfacing provenance and unknown-key warnings.
