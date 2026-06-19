# Accounts & Auth R1: Filesystem-Backed Account Registry

> Plan: cs-accounts-auth | Round: 1 of 4 | Complexity: L | Executor: prex (EF 1.5) | Generated: 2026-06-19 |
> Repo: /workspaces/claude-session

## Context

`claude-session` supports multiple named accounts, each with its own credential seed. This round builds
the filesystem-backed registry that lists/adds/inspects accounts and resolves their on-disk paths,
under the secure XDG state root established by `cs-isolation`. Later rounds populate seeds via managed
login and copy them into sessions. `cs-isolation` provides the `AccountId` newtype, the secure-dir
service, and `resolve_session_root`.

## Previous Rounds

`cs-foundation`: crate, plumbing, `--account` reserved global flag. `cs-isolation`: `AccountId`/
`GroupId`, secure dirs, `resolve_session_root` (`$XDG_STATE_HOME/claude-session/...`).
`cs-wrapper-runtime`: spawner + isolated child env. Expect all to exist.

## Scope of This Round

- IN scope: `services/account/registry.rs` (`Registry { root:
  $XDG_STATE_HOME/claude-session/accounts, last_account_path: .../state/last-account }`; `AccountEntry
  { id, dir, has_auth, last_used_at }`; `account_dir(id)`, `seed_path(id)` for the credential seed,
  `list()`, `add()`, `inspect()`); secure creation of `accounts/<name>/` (0700) via the isolation
  secure-dir service; reading/writing the `last-account` marker atomically.
- OUT of scope: managed login (round 2), resolver/gate/session-copy (round 3), CLI verbs (round 4).

## Current State

### Key Files

- `/workspaces/claude-session/src/services.rs` (+ `src/services/`) — add `account/` submodule with
  `registry.rs`.
- `/workspaces/claude-session/src/services/session/dir.rs` — reuse secure-dir helpers.
- `/workspaces/claude-session/src/domain/ids.rs` — reuse `AccountId`.

### Existing Patterns

Reference (codex-session `services/account/registry.rs`, inspiration only): `Registry { root:
accounts, last_account_path: state/last-account }`; `AccountEntry { id, dir, has_auth, last_used_at }`;
`account_dir` joins `root/<name>`; `group_auth_seed_path` joins `<account>/auth.json` (claude analog:
the credential seed file, e.g. `.credentials.json`); `list()` scans + sorts. Disk layout target:
`$XDG_STATE_HOME/claude-session/{state/last-account, accounts/<name>/}`.

## Implementation Steps

### First Step: Mark this round as started

In this plan's `queue-rounds.yaml`, set this round's (`item: account-registry`) `status` to `doing`.

### Step 1: Registry model

In `services/account/registry.rs`, define `Registry` and `AccountEntry`; implement `account_dir`,
`seed_path`, and secure creation of `accounts/<name>/`.

### Step 2: list/add/inspect

Implement `list()` (scan + sort), `add()` (create the account dir securely), and `inspect()`
(populate `has_auth`/`last_used_at`).

### Step 3: last-account marker

Read/write `state/last-account` atomically (tempfile+rename); expose a getter/setter.

### Step 4: Tests + errors

Add `AccountError` (thiserror) mapped into `AppError`; unit-test list/add/inspect and the last-account
roundtrip with `tempfile`.

### Final Step: Update the queue

1. In this plan's `queue-rounds.yaml`, set this round's (`item: account-registry`) `status` to `done`.

## Acceptance Criteria

- [ ] `Registry` resolves `accounts/<name>/` + seed path under the secure XDG state root; dirs are 0700.
- [ ] `list()`/`add()`/`inspect()` work; `has_auth` reflects seed presence.
- [ ] `last-account` is read/written atomically.
- [ ] `AccountError` maps into `AppError` with sysexits; tests pass.
- [ ] This plan's `queue-rounds.yaml` shows round `account-registry` as `done`.

## Next Round

Round 2 (`managed-login`) populates an account seed by running managed `claude login` (subscription
OAuth) in an isolated dir and securely copying the resulting credentials into the seed.
