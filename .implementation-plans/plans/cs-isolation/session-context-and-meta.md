# Isolation R3: Session Dir, Metadata, Context Integration & Cleanup

> Plan: cs-isolation | Round: 3 of 3 | Complexity: L | Executor: prex (EF 1.5) | Generated: 2026-06-19 | Repo: repository root

## Context

The child `claude` will later (in `cs-wrapper-runtime`) receive `CLAUDE_CONFIG_DIR=<session-dir>`. This round assembles the per-account/per-group session directory model from the secure primitives (round 1) and the identity derivation (round 2), writes session metadata, exposes the resolved session through `AppContext`, and adds conservative stale-session cleanup. After this round, `claude-session` can resolve a unique secure isolated directory for the current terminal — it just does not spawn into it yet.

## Previous Rounds

This plan round 1: `adapters/fs.rs`, secure-dir service, `resolve_session_root`. Round 2: `AccountId`/`GroupId` newtypes and the multiplexer-agnostic derivation chain. Expect both to exist and compile.

## Scope of This Round

- IN scope: `services/session/dir.rs` `session_dir(root, account, group)` building `accounts/<account>/groups/<group-id>/` securely step-by-step; `services/session/meta.rs` (atomic `session-meta.json` write via tempfile+persist; fields `{account, group_id, group_source, cwd,
  started_at(rfc3339), account_source}`); lazy session resolution in `AppContext` (resolve once, reuse); `services/session/cleanup.rs` (conservative age-based pruning of stale `groups/<id>/` dirs with no-follow symlink guards); `doctor` inspection helper surfacing the resolved session dir.
- OUT of scope: setting `CLAUDE_CONFIG_DIR` / spawning (`cs-wrapper-runtime`); real accounts (use `default`, replaced by `cs-accounts-auth`); credentials/config composition (later plans).

## Current State

### Key Files

- `src/services/session/dir.rs` — add `session_dir`.
- `src/services/session/meta.rs` — new.
- `src/services/session/cleanup.rs` — new.
- `src/context.rs` — add lazy session resolution.

### Existing Patterns

The path layout, modes, single-writer rule, and cleanup policy are specified in `docs/reference/xdg-storage.md`; the concepts are in `docs/explanation/session-isolation.md`.

`session_dir(root, account, group)` joins the account and group segments and secures **each level** with the round-1 helpers. Metadata is written atomically — a temporary file in the **same directory**, then rename, since rename is atomic only within a filesystem — at mode `0600`.

Cleanup prunes only directories under a `groups/` parent, never account directories, and **never follows symbolic links** out of the tree it is pruning. It is conservative by design: leaving a stale directory costs bytes, while deleting a live session costs the user their work. Keep it opt-in and reported.

For the timestamp use the `time` crate, not `chrono` — see the ruled-out list in `docs/reference/dependencies.md`. Resolve `started_at` once. The clock is an adapter so tests can inject it; see `docs/explanation/testing-strategy.md`.

## Implementation Steps

### First Step: Mark this round as started

In this plan's `queue-rounds.yaml`, set this round's (`item: session-context-and-meta`) `status` to `doing`.

### Step 1: Session dir

In `services/session/dir.rs`, add `session_dir(root, account, group)` securing each path segment under `accounts/<account>/groups/<group-id>/` (using round 1's secure-dir helpers). Use `account = "default"`.

### Step 2: Metadata

In `services/session/meta.rs`, atomically write `session-meta.json` (tempfile+persist) with account, group id + source, cwd, `started_at`, account source.

### Step 3: AppContext integration

Add lazy session resolution to `AppContext` (group-id derivation → root → `session_dir` → meta), so later commands request the resolved session without recomputation.

### Step 4: Cleanup

In `services/session/cleanup.rs`, add conservative stale-`groups/<id>/` pruning with no-follow guards; keep it opt-in/testable and never follow symlinks.

### Final Step: Update the queue

1. In this plan's `queue-rounds.yaml`, set this round's (`item: session-context-and-meta`) `status` to `done`.
2. All rounds are now done, so in the top-level `.implementation-plans/queue-plans.yaml` set this plan's (`item: cs-isolation`) `status` to `done`. Leave the plan directory in place.

## Acceptance Criteria

- [ ] The session dir path is `accounts/<account>/groups/<group>/` under the resolved XDG root, with each level secured.
- [ ] `session-meta.json` is written atomically and is valid JSON with the documented fields.
- [ ] `AppContext` exposes the resolved session info lazily (computed once).
- [ ] Cleanup never follows symlinks and only prunes stale group dirs.
- [ ] Tests pass (`cargo nextest run`, pre-commit profile).
- [ ] This plan's `queue-rounds.yaml` shows round `session-context-and-meta` as `done` and the top-level `queue-plans.yaml` shows `cs-isolation` as `done`.

## Next Round

This is the final round of this plan. `cs-wrapper-runtime` consumes the resolved session dir to inject `CLAUDE_CONFIG_DIR` into the child and spawn it with signal forwarding.
