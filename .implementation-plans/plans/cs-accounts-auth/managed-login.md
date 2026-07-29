# Accounts & Auth R2: Managed `claude login` & Hardened Seed Write

> Plan: cs-accounts-auth | Round: 2 of 4 | Complexity: L | Executor: prex (EF 1.5) | Generated: 2026-06-19 | Repo: repository root

## Context

Subscription auth is the primary, always-available path. `claude-session` authenticates an account by running a managed `claude login` in an isolated `CLAUDE_CONFIG_DIR`, then securely copying the resulting credentials into the account's seed (so they are not entangled with any other account or the user's native config). This round implements that managed-login flow and the hardened credential I/O. Round 1 built the registry; `cs-wrapper-runtime` provides the spawner used to run `claude login`.

## Previous Rounds

This plan round 1: `Registry`, account dirs, seed paths, `last-account`. `cs-wrapper-runtime`: `Spawner`/`spawn_and_wait`, child resolution, isolated child env. Expect both to exist.

## Scope of This Round

- IN scope: `services/account/login.rs` (run managed `claude login` with `CLAUDE_CONFIG_DIR` pointed at a fresh temp/isolated dir so it does not collide with other accounts or native config; on success, copy the produced credential file(s) — e.g. `.credentials.json` — into the account seed); `services/auth.rs` hardened file I/O (`ensure_owned_dir_0700`, `secure_file_read` with symlink/hardlink/ownership checks, atomic 0600 write); a `refresh` entrypoint that re-runs login for an existing account.
- OUT of scope: the resolver/gate + seed→session copy (round 3), CLI verbs (round 4), API-key fallback wiring (round 3).

## Current State

### Key Files

- `src/services/account/login.rs` — new.
- `src/services/auth.rs` — new (hardened credential I/O).
- `src/adapters/spawner.rs` — reuse to run `claude login`.
- `src/services/account/registry.rs` — write to the seed path.

### Existing Patterns

The credential model — a per-account seed, copied per session, with subscription login primary and token injection secondary — is recorded in `docs/decisions/0011-isolate-credentials-by-seed-and-session.md`. File modes, ownership checks, and atomic-write rules are in `docs/reference/xdg-storage.md`.

Run the login through the `Spawner` port with the configuration-directory variable pointed at a fresh scratch directory, so the flow cannot touch another account or the user's own configuration. On success, copy the produced credential file into the account seed at mode `0600`.

**Where the child stores credentials is a perishable, externally-owned fact** — a file inside its configuration directory on Linux, a system keychain elsewhere — tracked in `docs/reference/research-tracking.yaml`. Consume it defensively: a missing or unexpected credential file is a reported failure with a hint, never a panic and never a silent success.

Never mix a token helper with subscription credentials in one invocation, and never write a token to user-editable configuration or to any path a cache-clearing tool may remove.

## Implementation Steps

### First Step: Mark this round as started

In this plan's `queue-rounds.yaml`, set this round's (`item: managed-login`) `status` to `doing`.

### Step 1: Hardened auth I/O

In `services/auth.rs`, implement `ensure_owned_dir_0700`, `secure_file_read` (symlink/hardlink/owner checks), and atomic 0600 write.

### Step 2: Managed login

In `services/account/login.rs`, run `claude login` via the spawner with `CLAUDE_CONFIG_DIR` set to a fresh isolated dir; on success, securely copy the credential file(s) into the account seed.

### Step 3: Refresh

Add a `refresh(account)` that re-runs the login flow for an existing account, replacing the seed.

### Step 4: Tests

Test the seed-write path with a stubbed `claude login` that drops a fake `.credentials.json`; verify 0600 perms and that symlink/wrong-owner seeds are rejected.

### Final Step: Update the queue

1. In this plan's `queue-rounds.yaml`, set this round's (`item: managed-login`) `status` to `done`.

## Acceptance Criteria

- [ ] Managed `claude login` runs in an isolated dir and its credentials are copied into the account seed (0600), not into native config or user config.
- [ ] `secure_file_read`/the seed write reject symlinks/hardlinks/wrong-owner.
- [ ] `refresh(account)` re-authenticates and replaces the seed.
- [ ] Tests pass with a stubbed `claude login`.
- [ ] This plan's `queue-rounds.yaml` shows round `managed-login` as `done`.

## Next Round

Round 3 (`auth-gate-and-resolver`) adds account selection (flag > env > last/default), the auth gate that copies the seed into the session dir, the API-key/`ANTHROPIC_AUTH_TOKEN` fallback, and trust sync-back of `.claude.json`.
