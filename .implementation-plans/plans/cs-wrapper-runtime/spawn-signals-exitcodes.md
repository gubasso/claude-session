# Wrapper Runtime R2: Spawn-and-Wait, Signal Forwarding & Exit Codes

> Plan: cs-wrapper-runtime | Round: 2 of 3 | Complexity: L | Executor: prex (EF 1.5) | Generated: 2026-06-19 | Repo: /workspaces/claude-session

## Context

`claude-session` spawns the native `claude` as a child (not `exec`, because post-exit sync-back is required later) and must behave transparently: forward terminal signals to the child and propagate its exit status faithfully (child code N → N; signal death N → 128+N). This round implements `spawn_and_wait` with signal forwarding and exit-code mapping, replacing the minimal inherited-env spawn from `cs-foundation`. Round 1 produced the `Spawner` trait, child resolution, and the recursion guard.

## Previous Rounds

This plan round 1: `adapters/spawner.rs` with `Spawner`/`StdSpawner`, `resolve_child`, recursion guard, finalized `version`. `cs-foundation`: a minimal `pass_through.rs` spawn. Expect both to exist.

## Scope of This Round

- IN scope: `StdSpawner::spawn_and_wait(inv, pid_sink)` (build `std::process::Command`, manage env via the invocation's inherit/remove/set sets — env construction details land in round 3, here use a pass-through env), publish the child PID to an `Arc<AtomicI32>` for forwarding, wait, reset PID; `adapters/spawner.rs` signal forwarding (`signal-hook`: async-signal-safe flag registration + a dispatch thread forwarding SIGINT/SIGTERM/SIGHUP — plus SIGQUIT/SIGTSTP/SIGCONT/SIGUSR1/SIGUSR2/ SIGWINCH where applicable — to the child PID; emulate default handler before the child exists); exit-code mapping helper (child code → u8; signal death → 128+N; clamp/u8 with a warn on overflow); rewire `commands/pass_through.rs` and `commands/dispatch.rs` to use `spawn_and_wait` and return the mapped exit code through `AppError`/`main`.
- OUT of scope: isolated child env + `CLAUDE_CONFIG_DIR` injection + headroom seam (round 3); accounts/auth/config composition (later plans).

## Current State

### Key Files

- `/workspaces/claude-session/src/adapters/spawner.rs` — add `spawn_and_wait` + signal forwarding.
- `/workspaces/claude-session/src/commands/pass_through.rs` — adopt the robust spawn.
- `/workspaces/claude-session/src/commands/dispatch.rs` — route through `spawn_and_wait`.
- `/workspaces/claude-session/src/main.rs` — map the returned exit code (already wired in foundation).

### Existing Patterns

Reference (codex-session `adapters/spawner.rs`, inspiration only): `spawn_and_wait` publishes `child.id()` to a `pid_sink: &AtomicI32`, waits, resets to 0; `install_signal_forwarding(child_pid)` registers async-signal-safe flags for SIGINT/SIGTERM/SIGHUP and spawns a dispatch thread forwarding to the child PID (emulating the default handler when no child yet); `child_exit_code` clamps to u8 and warns on overflow. Wrapper-design exit-code rule (`06-cli-wrapper-design/process-and-posix.md`): child code N → N; signal N → 128+N. Add any blessed deps not yet present (`signal-hook`, `libc`/`rustix`) with **`cargo add`**, never by hand-editing `[dependencies]`.

## Implementation Steps

### First Step: Mark this round as started

In this plan's `queue-rounds.yaml`, set this round's (`item: spawn-signals-exitcodes`) `status` to `doing`.

### Step 1: spawn_and_wait

Implement `StdSpawner::spawn_and_wait` building the `Command`, publishing the child PID to the sink, waiting, and returning the `ExitStatus`. Keep env handling as a simple pass-through here (full construction in round 3).

### Step 2: Signal forwarding

Add `install_signal_forwarding` (async-signal-safe flags + dispatch thread) forwarding the terminal signal set to the child PID; emulate the default handler before the child exists; keep a guard alive for the spawn's lifetime.

### Step 3: Exit-code mapping + rewire

Add the exit-code helper (code N → N; signal N → 128+N; clamp to u8). Rewire `pass_through.rs`/ `dispatch.rs` to use `spawn_and_wait` and propagate the mapped code through `main`.

### Step 4: Tests

Integration-test (assert_cmd + stubbed `claude`) that the wrapper's exit code matches the child's, and that a stub which exits on a signal yields 128+N.

### Final Step: Update the queue

1. In this plan's `queue-rounds.yaml`, set this round's (`item: spawn-signals-exitcodes`) `status` to `done`.

## Acceptance Criteria

- [ ] The wrapper's exit code equals the child's exit code; a signal-killed child maps to 128+N (clamped to u8).
- [ ] Terminal signals are forwarded to the child PID (async-signal-safe; default handler emulated before spawn).
- [ ] `pass_through.rs` uses `spawn_and_wait`; the foundation's minimal spawn is fully replaced.
- [ ] Integration tests pass with a stubbed child.
- [ ] This plan's `queue-rounds.yaml` shows round `spawn-signals-exitcodes` as `done`.

## Next Round

Round 3 (`child-env-injection-and-headroom-seam`) builds the isolated child env, injects `CLAUDE_CONFIG_DIR` from the resolved session dir, exposes the `ANTHROPIC_BASE_URL` headroom/proxy seam, and adds end-to-end passthrough integration tests.
