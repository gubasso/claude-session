# Wrapper Runtime R2: Spawn-and-Wait, Signal Handling & Exit Status

> Plan: cs-wrapper-runtime | Round: 2 of 3 | Complexity: L | Executor: prex (EF 1.5) | Generated: 2026-06-19 | Repo: repository root

## Context

`claude-session` spawns the native `claude` as a child (not `exec`, because post-exit sync-back is required later) and must be **behaviourally indistinguishable** from `exec` in everything the user can observe: the same exit status, the same terminal behaviour, the same response to Ctrl-C. This round implements `spawn_and_wait` with the signal matrix and status propagation, replacing the minimal inherited-env spawn from `cs-foundation`. Round 1 produced the `Spawner` trait, child resolution, and the recursion guards.

## Previous Rounds

This plan round 1: `adapters/spawner.rs` with `Spawner`/`StdSpawner`, `resolve_child`, recursion guard, finalized `version`. `cs-foundation`: a minimal `pass_through.rs` spawn. Expect both to exist.

## Scope of This Round

- IN scope: `StdSpawner::spawn_and_wait(inv, pid_sink)` (build the command, manage env via the invocation's inherit/remove/set sets — env construction lands in round 3, here a pass-through env), publish the child process id for the signal machinery, wait, clear it; `adapters/spawner.rs` signal handling implementing the **matrix** in `docs/reference/process-runtime.md` via `signal-hook` — async-signal-safe flag registration plus a dispatch thread, forwarding only the signals the terminal does not broadcast to the group, re-raising `SIGSTOP` on the wrapper for `SIGTSTP`, and emulating the default action for a signal arriving before the child exists; status propagation (exit code unchanged; signal death reproduced by re-raise, `128 + N` clamped as fallback); rewire `commands/pass_through.rs` and `commands/dispatch.rs` to use `spawn_and_wait`.
- OUT of scope: isolated child env + `CLAUDE_CONFIG_DIR` injection + headroom seam (round 3); accounts/auth/config composition (later plans).

## Current State

### Key Files

- `src/adapters/spawner.rs` — add `spawn_and_wait` + signal forwarding.
- `src/commands/pass_through.rs` — adopt the robust spawn.
- `src/commands/dispatch.rs` — route through `spawn_and_wait`.
- `src/main.rs` — map the returned exit code (already wired in foundation).

### Existing Patterns

The process-group topology and the signal matrix are specified in `docs/reference/process-runtime.md`. **Read that matrix before writing any forwarding code — "forward every signal" is a bug here, not a safe default.**

The child **shares the wrapper's foreground process group**. A terminal-generated signal is therefore delivered by the kernel to every member, so the child already receives `SIGINT`, `SIGQUIT`, `SIGTSTP`, `SIGCONT`, and `SIGWINCH`. Forwarding those double-delivers, and a child that counts interrupts — one press to interrupt, two to quit — will read one keypress as two. The wrapper forwards only what the terminal does **not** broadcast: `SIGTERM`, `SIGHUP`, `SIGUSR1`, `SIGUSR2`.

Two further rules from the same page. On `SIGTSTP`, wait for the child to stop and then re-raise `SIGSTOP` on the wrapper itself, or the shell sees a live foreground process and withholds its prompt. And after a signal kills the child, **reproduce the child's fate** by resetting the signal to its default action and re-raising it on the wrapper, rather than exiting with a translated code — `128 + N` is the documented fallback, not the first choice. See `docs/reference/exit-codes.md`.

Handlers must be async-signal-safe: register a flag in the handler and do the work on a normal thread; never allocate, log, or lock inside one. Clear the published child process id **before** post-flight work, so a late signal cannot be forwarded to a reused process id.

Add any crates not yet present (`signal-hook`, `rustix`) with **`cargo add`**, never by hand-editing `[dependencies]`.

## Implementation Steps

### First Step: Mark this round as started

In this plan's `queue-rounds.yaml`, set this round's (`item: spawn-signals-exitcodes`) `status` to `doing`.

### Step 1: spawn_and_wait

Implement `StdSpawner::spawn_and_wait` building the `Command`, publishing the child PID to the sink, waiting, and returning the `ExitStatus`. Keep env handling as a simple pass-through here (full construction in round 3).

### Step 2: Signal handling

Add `install_signal_handling` implementing the matrix in `docs/reference/process-runtime.md` — **not** blanket forwarding. Forward only the signals the terminal does not broadcast to the group; leave the terminal-broadcast signals to the kernel. Handle `SIGTSTP` by re-raising `SIGSTOP` on the wrapper after the child stops. Use async-signal-safe flag registration plus a dispatch thread; emulate the default action for a signal arriving before the child exists; keep a guard alive for the spawn's lifetime.

### Step 3: Status propagation + rewire

Reproduce the child's outcome: an exit code passes through unchanged; signal death is reproduced by resetting the signal to its default and re-raising it on the wrapper, with `128 + N` (clamped) as the fallback where re-raise is impossible. Rewire `pass_through.rs` and `dispatch.rs` to use `spawn_and_wait`. A post-flight failure must **not** overwrite the child's status.

### Step 4: Tests

Integration-test with a stub child per `docs/reference/testing-and-quality.md`: the wrapper's exit code equals the child's; a signal-killed stub produces signal death rather than a plain exit; a single interrupt reaches the child exactly **once** (the double-delivery regression); a forwarded `SIGTERM` reaches the child.

### Final Step: Update the queue

1. In this plan's `queue-rounds.yaml`, set this round's (`item: spawn-signals-exitcodes`) `status` to `done`.

## Acceptance Criteria

- [ ] The wrapper's exit code equals the child's; a signal-killed child produces signal death on the wrapper, or `128 + N` clamped where re-raise is impossible.
- [ ] The signal matrix in `docs/reference/process-runtime.md` is implemented exactly: terminal-broadcast signals are **not** forwarded, and a single interrupt reaches the child exactly once (test-verified).
- [ ] `SIGTERM` and `SIGHUP` are forwarded; handlers are async-signal-safe; the default action is emulated for a signal arriving before the child exists.
- [ ] A post-flight failure does not overwrite the child's exit status.
- [ ] `pass_through.rs` uses `spawn_and_wait`; the foundation's minimal spawn is fully replaced.
- [ ] Integration tests pass with a stubbed child.
- [ ] This plan's `queue-rounds.yaml` shows round `spawn-signals-exitcodes` as `done`.

## Next Round

Round 3 (`child-env-injection-and-headroom-seam`) builds the isolated child env, injects `CLAUDE_CONFIG_DIR` from the resolved session dir, exposes the `ANTHROPIC_BASE_URL` headroom/proxy seam, and adds end-to-end passthrough integration tests.
