# Wrapper Runtime R1: Spawner Port, Child Resolution & Recursion Guard

> Plan: cs-wrapper-runtime | Round: 1 of 3 | Complexity: L | Executor: prex (EF 1.5) | Generated: 2026-06-19 |
> Repo: /workspaces/claude-session

## Context

`claude-session` must reliably locate and validate the native `claude` binary before spawning it, and
must never re-invoke itself (a shim/recursion risk if installed under a colliding name or via PATH
ordering). This round defines the hexagonal `Spawner` port and the child-binary resolution chain with
a recursion guard, and finalizes the `version` verb (our version + the resolved child path/version).
`cs-foundation` provides the crate, `AppContext`, `AppError`, `Ui`, and a minimal inherited-env spawn
in `pass_through.rs` that this plan will harden. `cs-isolation` provides the resolved session dir
(consumed in round 3).

## Previous Rounds

`cs-foundation`: crate tree, plumbing, clap passthrough skeleton with a minimal spawn and a `version`
stub; `which` may already be a dep. `cs-isolation`: secure dirs, group identity, `session_dir`.
Expect these to exist and compile.

## Scope of This Round

- IN scope: `adapters/spawner.rs` — a `Spawner` trait (port) with `resolve_child(&ChildConfig)`,
  `child_version_line`, `spawn_and_wait` (signature only; impl in round 2), and `exec`; a default
  `StdSpawner` implementing `resolve_child` (resolution chain) and `child_version_line`; the binary
  resolution chain (`$CLAUDE_SESSION_CHILD_BIN` → config `child_bin` → `PATH` via `which` →
  optional vendor path) with an executability check; the recursion guard
  (`CLAUDE_SESSION_REENTRY=1` marker + `current_exe().canonicalize()` self-check) refusing to resolve
  the wrapper itself; finalize `commands/version.rs` to print claude-session's version plus the
  resolved child path + `claude --version`.
- OUT of scope: the actual spawn/wait + signal forwarding (round 2); child env construction +
  `CLAUDE_CONFIG_DIR` injection (round 3); accounts (`cs-accounts-auth`).

## Current State

### Key Files

- `/workspaces/claude-session/src/adapters.rs` (+ `src/adapters/`) — add `spawner.rs`.
- `/workspaces/claude-session/src/commands/pass_through.rs` — currently a minimal spawn; will adopt the
  `Spawner` resolution here, full spawn in round 2.
- `/workspaces/claude-session/src/commands/version.rs` — finalize child path/version reporting.
- `/workspaces/claude-session/src/error.rs` — add `ChildNotFound`/`ChildNotExecutable` mappings.

### Existing Patterns

Reference `Spawner` (codex-session `adapters/spawner.rs`, inspiration only): trait with `resolve_child`,
`child_version_line`, `spawn_and_wait(inv, pid_sink: &AtomicI32)`, `exec`; `StdSpawner` default impl;
resolution validates executability and guards against resolving a symlink to self. Wrapper-design
binary-resolution + recursion-guard rules (`cli-design/06-cli-wrapper-design/process-and-posix.md`):
lookup order `$<APP>_CHILD_BIN` → config → `PATH` → vendor; recursion guard via marker env + inode/path
self-check. sysexits: `127` child-not-found, `126` not-executable.

## Implementation Steps

### First Step: Mark this round as started

In this plan's `queue-rounds.yaml`, set this round's (`item: child-resolution`) `status` to `doing`.

### Step 1: Spawner trait

In `adapters/spawner.rs`, define the `Spawner` trait and a `StdSpawner` skeleton. Add `ChildConfig`
(holds optional explicit `child_bin`).

### Step 2: Resolution chain + recursion guard

Implement `resolve_child`: `$CLAUDE_SESSION_CHILD_BIN` → config `child_bin` → `which("claude")` →
optional vendor path; verify the resolved path is executable; reject it if it canonicalizes to
`current_exe()` or if `CLAUDE_SESSION_REENTRY` is already set (return `ChildNotFound`/a recursion
error). Map failures to sysexits 127/126.

### Step 3: version verb

Finalize `commands/version.rs` to print claude-session's own version and the resolved child path plus
`claude --version` output (via `child_version_line`).

### Step 4: Tests

Unit/integration test resolution precedence (env over PATH) and the recursion guard, using a stubbed
`claude` binary and `CLAUDE_SESSION_CHILD_BIN`.

### Final Step: Update the queue

1. In this plan's `queue-rounds.yaml`, set this round's (`item: child-resolution`) `status` to `done`.

## Acceptance Criteria

- [ ] `resolve_child` honors `$CLAUDE_SESSION_CHILD_BIN` over config over `PATH`; non-executable →
      sysexits 126; not-found → 127.
- [ ] The recursion guard refuses to resolve the wrapper itself (marker env + canonicalized self-check).
- [ ] `claude-session version` prints our version + the resolved child path + `claude --version`.
- [ ] Tests pass with a stubbed child binary.
- [ ] This plan's `queue-rounds.yaml` shows round `child-resolution` as `done`.

## Next Round

Round 2 (`spawn-signals-exitcodes`) implements `spawn_and_wait` with signal forwarding and exit-code
mapping, replacing the foundation's minimal spawn.
