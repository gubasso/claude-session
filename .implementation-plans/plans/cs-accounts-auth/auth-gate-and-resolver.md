# Accounts & Auth R3: Resolver, Auth Gate, Session Copy & Fallback

> Plan: cs-accounts-auth | Round: 3 of 4 | Complexity: L | Executor: prex (EF 1.5) | Generated: 2026-06-19 | Repo: repository root

## Context

Before spawning `claude`, `claude-session` selects an account, ensures its credential seed exists, and copies that seed into the isolated session dir so the child authenticates as that account. It also offers an API-key / `ANTHROPIC_AUTH_TOKEN` fallback for headless/CI, and syncs `.claude.json` project trust state back to the seed after the session. This round implements the resolver, the auth gate, the seed→session copy, the fallback, and trust sync-back. Round 1 built the registry; round 2 built managed login + hardened I/O; `cs-isolation` provides the session dir; `cs-wrapper-runtime` provides the spawn.

## Previous Rounds

This plan R1: `Registry`. R2: managed login + `services/auth.rs`. `cs-isolation`: resolved session dir in `AppContext`. `cs-wrapper-runtime`: isolated spawn (`pass_through.rs` builds the `ChildInvocation`). Expect all to exist.

## Scope of This Round

- IN scope: `services/account/resolver.rs` (`AccountIntent { Pinned{id, source}, ... }`; priority CLI `--account` flag > env `CLAUDE_SESSION_ACCOUNT` > `last-account`/`default`); `services/account/gate.rs` (`ensure`: pinned account → if seed exists, copy seed → session dir; if missing, run managed login or return a clear error); the seed→session credential copy (atomic, into the `cs-isolation` session dir); the API-key/`ANTHROPIC_AUTH_TOKEN` fallback path injected via the child env when no managed seed is desired; `services/trust_sync.rs` (post-flight conservative sync of `.claude.json` `[projects]` trust/onboarding state back to the seed under a lock); wiring the gate into `commands/pass_through.rs` before spawn and trust sync-back after.
- OUT of scope: CLI `account` verbs (round 4); quota/failover (out of scope for v1); config composition (`cs-config-composition`).

## Current State

### Key Files

- `src/services/account/resolver.rs` — new.
- `src/services/account/gate.rs` — new.
- `src/services/trust_sync.rs` — new.
- `src/commands/pass_through.rs` — call the gate before spawn, sync after.

### Existing Patterns

Account selection follows the same shape as every other precedence chain in this project — an explicit flag, then the environment, then the persisted default — and the resolved **source** is recorded so `doctor` can explain the choice. The flag and environment names are in `docs/reference/cli-surface.md` and `docs/reference/configuration.md`.

The seed-to-session copy is the mechanism recorded in `docs/decisions/0011-isolate-credentials-by-seed-and-session.md`: each session gets an independent copy the child may rewrite freely, so a concurrent session cannot observe a half-written state or lose a refresh. The copy is atomic and idempotent.

Token injection (`ANTHROPIC_API_KEY` / `ANTHROPIC_AUTH_TOKEN`) is a **first-class secondary path**, not an afterthought — the login flow needs a browser, so containers and continuous integration depend on it. The two paths are never mixed within one invocation.

Trust sync-back runs **post-flight**, after the child exits, under a lock, merging conservatively: a concurrent session's newer state is not this session's to discard. A sync failure is reported and **must not change the child's exit status** — see `docs/reference/process-runtime.md`.

## Implementation Steps

### First Step: Mark this round as started

In this plan's `queue-rounds.yaml`, set this round's (`item: auth-gate-and-resolver`) `status` to `doing`.

### Step 1: Resolver

In `services/account/resolver.rs`, implement the priority chain (flag > env > last/default) returning a pinned account + source.

### Step 2: Auth gate + session copy

In `services/account/gate.rs`, implement `ensure`: resolve the account, verify the seed (login if missing or error with a hint), and atomically copy the seed into the isolated session dir.

### Step 3: API-key fallback

When configured for token/API-key auth, inject `ANTHROPIC_API_KEY`/`ANTHROPIC_AUTH_TOKEN` into the child env instead of a managed seed (secondary path).

### Step 4: Trust sync-back + wiring

In `services/trust_sync.rs`, conservatively sync `.claude.json` `[projects]` state back to the seed post-flight under a lock. Wire the gate before spawn and sync after, in `pass_through.rs`.

### Final Step: Update the queue

1. In this plan's `queue-rounds.yaml`, set this round's (`item: auth-gate-and-resolver`) `status` to `done`.

## Acceptance Criteria

- [ ] Account selection honors flag > env > last/default; the resolved source is recorded.
- [ ] The gate copies the seed into the isolated session dir (atomic) or runs login / errors clearly when the seed is missing.
- [ ] The API-key/`ANTHROPIC_AUTH_TOKEN` fallback injects auth into the child env when configured.
- [ ] Trust sync-back updates the seed conservatively under a lock without clobbering newer state.
- [ ] Tests pass with stubbed `claude` + seed fixtures.
- [ ] This plan's `queue-rounds.yaml` shows round `auth-gate-and-resolver` as `done`.

## Next Round

Round 4 (`account-commands`) exposes `account add|list|current|remove|refresh` with `--format json` and secret redaction, and wires account state into `doctor`.
