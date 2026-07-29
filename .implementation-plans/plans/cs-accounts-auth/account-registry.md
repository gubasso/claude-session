# Accounts & Auth R1: Filesystem-Backed Account Registry

> Plan: cs-accounts-auth | Round: 1 of 4 | Complexity: L | Executor: prex (EF 1.5) | Generated: 2026-06-19 | Repo: repository root

## Context

`claude-session` supports multiple named accounts, each with its own credential seed. This round builds the filesystem-backed registry that lists/adds/inspects accounts and resolves their on-disk paths, under the secure XDG state root established by `cs-isolation`. Later rounds populate seeds via managed login and copy them into sessions. `cs-isolation` provides the `AccountId` newtype, the secure-dir service, and `resolve_session_root`.

## Previous Rounds

`cs-foundation`: crate, plumbing, `--account` reserved global flag. `cs-isolation`: `AccountId`/ `GroupId`, secure dirs, `resolve_session_root` (`$XDG_STATE_HOME/claude-session/...`). `cs-wrapper-runtime`: spawner + isolated child env. Expect all to exist.

## Scope of This Round

- IN scope: `services/account/registry.rs` (`Registry { root:
  $XDG_STATE_HOME/claude-session/accounts, last_account_path: .../state/last-account }`; `AccountEntry
  { id, dir, has_auth, last_used_at }`; `account_dir(id)`, `seed_path(id)` for the credential seed, `list()`, `add()`, `inspect()`); secure creation of `accounts/<name>/` (0700) via the isolation secure-dir service; reading/writing the `last-account` marker atomically.
- OUT of scope: managed login (round 2), resolver/gate/session-copy (round 3), CLI verbs (round 4).

## Current State

### Key Files

- `src/services.rs` (+ `src/services/`) — add `account/` submodule with `registry.rs`.
- `src/services/session/dir.rs` — reuse secure-dir helpers.
- `src/domain/ids.rs` — reuse `AccountId`.

### Existing Patterns

The on-disk layout, the writer of each artifact, and its mode are specified in the artifact table in `docs/reference/xdg-storage.md`. Account directories are `0700`; the credential seed and the last-account marker are `0600`. Every one of these artifacts has exactly **one writer** — this subsystem — which is what makes concurrent sessions on the same account safe.

`AccountId` validation reuses the newtype from `cs-isolation`: charset `[a-z0-9_-]`, a lowercase-ASCII or digit first character, at most 32 bytes, and a value that fails validation is rejected rather than truncated.

Every write that must not be observed half-finished — the seed, the last-account marker — goes through a temporary file **in the same directory** followed by a rename, since rename is atomic only within a filesystem.

Credentials live only in the secure state tree: never in user-editable configuration, never in cache, never in the runtime base. See `docs/decisions/0011-isolate-credentials-by-seed-and-session.md`.

## Implementation Steps

### First Step: Mark this round as started

In this plan's `queue-rounds.yaml`, set this round's (`item: account-registry`) `status` to `doing`.

### Step 1: Registry model

In `services/account/registry.rs`, define `Registry` and `AccountEntry`; implement `account_dir`, `seed_path`, and secure creation of `accounts/<name>/`.

### Step 2: list/add/inspect

Implement `list()` (scan + sort), `add()` (create the account dir securely), and `inspect()` (populate `has_auth`/`last_used_at`).

### Step 3: last-account marker

Read/write `state/last-account` atomically (tempfile+rename); expose a getter/setter.

### Step 4: Tests + errors

Add `AccountError` (thiserror) mapped into `AppError`; unit-test list/add/inspect and the last-account roundtrip with `tempfile`.

### Final Step: Update the queue

1. In this plan's `queue-rounds.yaml`, set this round's (`item: account-registry`) `status` to `done`.

## Acceptance Criteria

- [ ] `Registry` resolves `accounts/<name>/` + seed path under the secure XDG state root; dirs are 0700.
- [ ] `list()`/`add()`/`inspect()` work; `has_auth` reflects seed presence.
- [ ] `last-account` is read/written atomically.
- [ ] `AccountError` maps into `AppError` with sysexits; tests pass.
- [ ] This plan's `queue-rounds.yaml` shows round `account-registry` as `done`.

## Next Round

Round 2 (`managed-login`) populates an account seed by running managed `claude login` (subscription OAuth) in an isolated dir and securely copying the resulting credentials into the seed.
