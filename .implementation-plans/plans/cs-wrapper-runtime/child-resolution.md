# Wrapper Runtime R1: Spawner Port, Child Resolution & Recursion Guard

> Plan: cs-wrapper-runtime | Round: 1 of 3 | Complexity: L | Executor: prex (EF 1.5) | Generated: 2026-06-19 | Repo: repository root

## Context

`claude-session` must reliably locate and validate the native `claude` binary before spawning it, and must never re-invoke itself (a shim/recursion risk if installed under a colliding name or via PATH ordering). This round defines the `Spawner` port and the child-binary resolution ladder with both recursion guards, and finalizes the `version` verb (our version plus the resolved child path and version). `cs-foundation` provides the crate, `AppContext`, `AppError`, `Ui`, and a minimal inherited-env spawn in `pass_through.rs` that this plan will harden. `cs-isolation` provides the resolved account directory and composed-settings entry (consumed in round 3).

## Previous Rounds

`cs-foundation`: crate tree, plumbing, clap passthrough skeleton with a minimal spawn and a `version` stub; `which` may already be a dep. `cs-isolation`: secure dirs, validated identifiers, the composed-settings entry key. Expect these to exist and compile.

## Scope of This Round

- IN scope: `adapters/spawner.rs` — a `Spawner` trait (port) with `resolve_child(&ChildConfig)`, `child_version_line`, and `spawn_and_wait` (signature only; implementation in round 2); a default `StdSpawner` implementing `resolve_child` and `child_version_line`; the resolution ladder (`$CLAUDE_SESSION_CHILD_BIN` → config `child_bin` → `PATH` via `which` → optional vendor path) with existence, file-kind, and executability checks per candidate; both recursion guards (`CLAUDE_SESSION_REENTRY=1` marker **and** `current_exe().canonicalize()` self-check); finalize `commands/version.rs` to print claude-session's version plus the resolved child path and version, degrading to a reported resolution failure rather than aborting when the child cannot be found.
- OUT of scope: the actual spawn/wait + signal forwarding (round 2); child env construction + `CLAUDE_CONFIG_DIR` injection (round 3); accounts (`cs-accounts-auth`).

## Current State

### Key Files

- `src/adapters.rs` (+ `src/adapters/`) — add `spawner.rs`.
- `src/commands/pass_through.rs` — currently a minimal spawn; will adopt the `Spawner` resolution here, full spawn in round 2.
- `src/commands/version.rs` — finalize child path/version reporting.
- `src/error.rs` — add `ChildNotFound`/`ChildNotExecutable` mappings.

### Existing Patterns

The resolution ladder, its per-candidate validation, and both recursion guards are specified in `docs/reference/process-runtime.md`; the reasoning is in `docs/explanation/wrapper-model.md`.

Two details are easy to get wrong. **The first candidate that exists wins** — a candidate that exists but fails validation is an error, not a reason to try the next rung, because falling through would silently run a different binary than the user named. And **both** guards are required: the marker variable is defeated by an environment scrubbed between invocations, and the canonicalized self-check is defeated by a _copy_ of the wrapper rather than a link to it.

Do **not** add an `exec` method to the trait. This project spawns and waits; `exec` is recorded as rejected in `docs/decisions/ADR-0004-spawn-and-wait-child-supervision.md`, and a trait method nobody may call is an invitation.

Codes come from `docs/reference/exit-codes.md` and are not decided here. Note that the recursion refusal is **not** a not-found: resolution succeeded and produced the wrong binary, so it carries its own `err.kind` and its own code.

## Implementation Steps

### First Step: Mark this round as started

In this plan's `queue-rounds.yaml`, set this round's (`item: child-resolution`) `status` to `doing`.

### Step 1: Spawner trait

In `adapters/spawner.rs`, define the `Spawner` trait and a `StdSpawner` skeleton. Add `ChildConfig` (holds the optional explicit `child_bin`). The trait is the seam that makes the wrapper testable without spawning real processes — see `docs/explanation/testing-strategy.md` — so keep it narrow and free of process-specific types in its signatures.

### Step 2: Resolution chain + recursion guard

Implement `resolve_child`: `$CLAUDE_SESSION_CHILD_BIN` → config `child_bin` → `which("claude")` → optional vendor path. **The first candidate that exists wins**; validate it (regular file or symlink to one, executable by the current user) and fail rather than falling through. Reject it if it canonicalizes to `current_exe()`, or if `CLAUDE_SESSION_REENTRY` is already set — both guards, not either. Map failures per `docs/reference/exit-codes.md`.

### Step 3: version verb

Finalize `commands/version.rs` to print claude-session's own version and the resolved child path plus `claude --version` output (via `child_version_line`).

### Step 4: Tests

Unit/integration test resolution precedence (env over PATH) and the recursion guard, using a stubbed `claude` binary and `CLAUDE_SESSION_CHILD_BIN`.

### Final Step: Update the queue

1. In this plan's `queue-rounds.yaml`, set this round's (`item: child-resolution`) `status` to `done`.

## Acceptance Criteria

- [ ] `resolve_child` honors `$CLAUDE_SESSION_CHILD_BIN` over config over `PATH`; non-executable → sysexits 126; not-found → 127.
- [ ] The recursion guard refuses to resolve the wrapper itself (marker env + canonicalized self-check).
- [ ] `claude-session version` prints our version + the resolved child path + `claude --version`.
- [ ] Tests pass with a stubbed child binary.
- [ ] This plan's `queue-rounds.yaml` shows round `child-resolution` as `done`.

## Next Round

Round 2 (`spawn-signals-exitcodes`) implements `spawn_and_wait` with the signal matrix and exit-status propagation, replacing the foundation's minimal spawn.
