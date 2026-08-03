# Accounts & Auth R1: Account Discovery, Mode Metadata & Selection

> Plan: cs-accounts-auth | Round: 1 of 4 | Complexity: L | Executor: prex (EF 1.5) | Generated: 2026-06-19 | Repo: repository root

## Context

`claude-session` supports multiple named accounts. An account is a directory under the state base; there is no index file, so enumerating accounts means reading that directory. This round builds the account store — discovery, the per-account mode metadata, the last-used marker, and the hardened file I/O they all go through — plus the selection resolver every later round reads. `cs-isolation` provides the `AccountId` newtype, the secure-dir service, and the resolved state root.

## Previous Rounds

`cs-foundation`: crate, plumbing, `--account` reserved global flag. `cs-isolation`: `AccountId`/`ProfileId`, secure dirs, state-root resolution. `cs-wrapper-runtime`: spawner + child env construction. Expect all to exist.

## Scope of This Round

- IN scope: `services/account/store.rs` (discover `accounts/<account>/`; create it and its `config/` securely; read and atomically write `auth-mode.json`; read and atomically write the `state/last-account` marker); `services/auth.rs` hardened file I/O (`ensure_owned_dir_0700`, `secure_file_read` with symlink/hardlink/ownership checks, atomic `0600` write); `services/account/resolver.rs` (the selection ladder, returning the account **and the rung that supplied it**); an `AccountError` mapped into `AppError`.
- OUT of scope: `account login` in either mode (round 2), launch-time environment and precedence (round 3), the CLI verbs (round 4).

## Current State

### Key Files

- `src/services.rs` (+ `src/services/`) — add the `account/` submodule.
- `src/services/session/dir.rs` — reuse the secure-dir helpers.
- `src/domain/ids.rs` — reuse `AccountId`.

### Existing Patterns

What an account is, what `auth-mode.json` records, and the selection ladder are specified in `docs/reference/accounts.md`. The on-disk layout, each artifact's writer, and its mode are in the artifact table in `docs/reference/xdg-storage.md`. The precedence rungs above the last-used marker belong to `docs/reference/configuration.md`; this subsystem appends one rung below them rather than defining a chain of its own.

**There is no registry index**, and that is the load-bearing property: a second source of truth for "which accounts exist" drifts from the filesystem the first time a directory is created or removed outside the wrapper. Discovery reads the directory every time.

The ownership boundary is the other rule this round must not blur. The wrapper owns account selection, mode metadata, and any stored token. The child owns everything below the account's `config/`, including `.credentials.json`. **The wrapper never reads, copies, writes, refreshes, synchronizes, or fingerprints a child credential**, and caches no child authentication state — `status` may `stat` the path, and that is all.

`AccountId` validation reuses the newtype from `cs-isolation`: charset `[a-z0-9_-]`, a lowercase-ASCII or digit first character, at most 32 bytes, and a value that fails validation is rejected rather than truncated.

Every write that must not be observed half-finished — mode metadata, the last-used marker — goes through a temporary file **in the same directory** followed by a rename, since rename is atomic only within a filesystem.

## Implementation Steps

### First Step: Mark this round as started

In this plan's `queue-rounds.yaml`, set this round's (`item: account-discovery`) `status` to `doing`.

### Step 1: Hardened I/O

In `services/auth.rs`, implement `ensure_owned_dir_0700`, `secure_file_read` (symlink/hardlink/owner checks), and the atomic `0600` write every later step goes through.

### Step 2: Account store

In `services/account/store.rs`, implement discovery of `accounts/<account>/`, secure creation of an account directory and its `config/`, and typed read/write of `auth-mode.json` (`mode`, `recorded_at`, and `fingerprint` in token mode only).

### Step 3: Last-used marker

Read and write `state/last-account` atomically; expose a getter and setter. Removing the selected account clears it.

### Step 4: Resolver

In `services/account/resolver.rs`, implement the ladder from `docs/reference/accounts.md`, returning the resolved account together with the rung that answered, so later rounds can report provenance rather than re-deriving it.

### Step 5: Tests + errors

Add `AccountError` (thiserror) mapped into `AppError` per `docs/reference/exit-codes.md`; unit-test discovery, the mode-metadata round trip, the marker round trip, and each resolver rung with `tempfile`.

### Final Step: Update the queue

1. In this plan's `queue-rounds.yaml`, set this round's (`item: account-discovery`) `status` to `done`.

## Acceptance Criteria

- [ ] Accounts are enumerated by reading `accounts/` — no index file is written or read.
- [ ] Account directories are `0700`; `auth-mode.json` and the last-used marker are `0600` and written atomically.
- [ ] No code path reads, writes, or fingerprints a file below an account's `config/`.
- [ ] The resolver honors every rung and reports which one supplied the answer.
- [ ] `AccountError` maps into `AppError` with the kinds `docs/reference/accounts.md` credits; tests pass.
- [ ] This plan's `queue-rounds.yaml` shows round `account-discovery` as `done`.

## Next Round

Round 2 (`account-login`) implements `account login` in both stored modes: launching the child's own login into the account's `config/`, and ingesting a long-lived subscription token.
