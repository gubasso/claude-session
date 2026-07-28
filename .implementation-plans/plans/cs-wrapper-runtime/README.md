# claude-session — Wrapper Runtime (process, spawn, signals, child env)

> Complexity: L | Rounds: 3 | Generated: 2026-06-19 | Repo: /workspaces/claude-session

## Problem Statement

`claude-session` ultimately executes the native `claude` binary with an isolated config dir, correct env, preserved argv, robust signal forwarding, faithful exit-code behavior, and a clean seam for headroom or any proxy via `ANTHROPIC_BASE_URL`. This plan owns the process/spawn subsystem — the hexagonal `Spawner` port, child-binary resolution with a recursion guard, spawn-and-wait with signal forwarding and exit-code mapping, and isolated child-env construction — replacing the minimal inherited-env spawn that `cs-foundation` shipped. This is a coherent, reusable domain (the reference's `adapters/spawner.rs` + `domain/child_invocation.rs`), kept separate from "compute the session location" (`cs-isolation`). Depends on `cs-isolation` (it consumes the resolved session dir) and, transitively, `cs-foundation`.

## Strategy

Three rounds. R1 builds child resolution (the binary lookup chain + recursion guard) and the `Spawner` trait, and finalizes the `version` verb. R2 implements spawn-and-wait with signal forwarding and exit-code mapping, replacing the foundation's minimal spawn. R3 builds the isolated child-env (`ChildInvocation`/`ChildEnv` — scrub internal vars, set `CLAUDE_CONFIG_DIR`), wires the isolation plan's session dir into the spawn, exposes the injectable `ANTHROPIC_BASE_URL` headroom/proxy seam, and adds end-to-end passthrough integration tests.

## Rounds

1. `child-resolution.md` — `Spawner` trait, child-binary resolution chain, recursion guard, `version`.
2. `spawn-signals-exitcodes.md` — spawn-and-wait, signal forwarding, child exit-code mapping.
3. `child-env-injection-and-headroom-seam.md` — `ChildEnv` scrubbing + `CLAUDE_CONFIG_DIR` injection + `ANTHROPIC_BASE_URL` seam + e2e passthrough tests.

## Execution Commands

```bash
# Execute the next todo round (executor reads queue-rounds.yaml, runs the first todo round, then stops):
/prex -ar @.implementation-plans/plans/cs-wrapper-runtime/

# Or target a specific round file directly:
/prex -ar .implementation-plans/plans/cs-wrapper-runtime/child-resolution.md
```

## Execution Discipline

**Rounds must be executed one at a time.** Each round is a self-contained unit of work designed for a single `/prex` session. Do not implement multiple rounds in one session.

When `/prex` is pointed at this directory or this `README.md`, it MUST:

1. Read this plan's `queue-rounds.yaml`.
2. Find the first round with status `todo`.
3. Set that round's `status` to `doing`, execute ONLY that round, then set it to `done` and stop.
4. End the session — a fresh `/prex` session is launched for any subsequent round.

## Decisions & Constraints

- `Executor: prex (EF 1.5)`.
- **Spawn-and-wait, not `exec`** — post-exit work (credential sync-back, trust sync in `cs-accounts-auth`) requires control to return after the child exits.
- **Child-binary resolution** (highest first): `$CLAUDE_SESSION_CHILD_BIN` → config entry (`child_bin`) → `PATH` search → optional bundled vendor path.
- **Recursion guard**: marker env `CLAUDE_SESSION_REENTRY=1` + `current_exe().canonicalize()` self-check so the wrapper never re-invokes itself as the child.
- **Exit codes**: child code N → wrapper exits N; child died from signal N → wrapper exits 128+N (SIGINT→130, SIGTERM→143, SIGKILL→137); clamp to u8.
- **Signal forwarding**: forward at least SIGINT/SIGTERM/SIGHUP (and SIGQUIT/SIGTSTP/SIGCONT/ SIGUSR1/SIGUSR2/SIGWINCH per the wrapper spec) to the child via a published child PID.
- **Child env**: inherit parent env, REMOVE internal `CLAUDE_SESSION_*` keys (except the intentional `REENTRY` marker), SET `CLAUDE_CONFIG_DIR=<session-dir>`. Argv forwarded verbatim as `OsString`.
- **headroom seam**: child env (esp. `ANTHROPIC_BASE_URL`) must be composeable/injectable so headroom (or any proxy) can front `claude`. Implement NO internal compression.

## Rejected Alternatives

- **`execvp` as the default** — rejected; auth/trust sync-back must run after the child exits.
- **Internal headroom/compression logic** — rejected; only provide the env/wrap seam.
- **Parsing the child's grammar to rewrite flags** — rejected; default to verbatim pass-through, claim only a denylist of wrapper-owned flags.

## Risks & Edge Cases

- Signal forwarding is platform-sensitive; keep the Unix implementation focused, async-signal-safe, and tested where possible.
- Env leakage: ensure no internal `CLAUDE_SESSION_*` value reaches the child except the intentional `REENTRY` marker.
- Non-UTF-8 argv: preserve `OsString` end-to-end.
- Child binary missing/non-executable: map to sysexits 127/126 with a four-part error + hint.

## Completion

When all rounds are done, set each round `done` in this plan's `queue-rounds.yaml` and set this plan `done` in the top-level `.implementation-plans/queue-plans.yaml`. Nothing moves on disk.
