# claude-session — Wrapper Runtime (process, spawn, signals, child env)

> Complexity: L | Rounds: 3 | Generated: 2026-06-19 | Repo: repository root

## Problem Statement

`claude-session` ultimately executes the native `claude` binary with the right configuration directory, correct env, preserved argv, correct signal semantics, faithful exit status, and a clean seam for any fronting proxy. This plan owns the process/spawn subsystem — the `Spawner` port, child-binary resolution with two recursion guards, spawn-and-wait with the signal matrix and status propagation, and child-env and argv construction — replacing the minimal inherited-env spawn that `cs-foundation` shipped. It is kept separate from "compute the storage location and identifiers" (`cs-isolation`). The contracts it implements are in `docs/reference/process-runtime.md` and `docs/reference/exit-codes.md`; the reasoning is in `docs/explanation/wrapper-model.md`. Depends on `cs-isolation` (it consumes the resolved account directory and composed-settings entry) and, transitively, `cs-foundation`.

## Strategy

Three rounds. R1 builds child resolution (the lookup ladder + both recursion guards) and the `Spawner` trait, and finalizes the `version` verb. R2 implements spawn-and-wait with the signal matrix and status propagation, replacing the foundation's minimal spawn. R3 builds the child environment (`ChildInvocation`/`ChildEnv` — scrub internal vars, set the account-scoped `CLAUDE_CONFIG_DIR`), builds the wrapper-owned argv prefix around the untouched user suffix, exposes the general environment-injection seam, and adds end-to-end passthrough integration tests.

## Rounds

1. `child-resolution.md` — `Spawner` trait, child-binary resolution chain, recursion guard, `version`.
2. `spawn-signals-exitcodes.md` — spawn-and-wait, the signal matrix, child exit-status propagation.
3. `child-env-injection-and-headroom-seam.md` — `ChildEnv` scrubbing + `CLAUDE_CONFIG_DIR` injection + the argv-prefix seam + the general env-injection seam + e2e passthrough tests.

## Execution Commands

Any executor following [the contract](../../README.md#the-executor-contract) can run these rounds. `/prex` is the one used to generate them, shown here as a worked example:

```bash
# Execute the next todo round (executor reads queue-rounds.yaml, runs the first todo round, then stops):
/prex -ar @.implementation-plans/plans/cs-wrapper-runtime/

# Or target a specific round file directly:
/prex -ar .implementation-plans/plans/cs-wrapper-runtime/child-resolution.md
```

## Execution Discipline

Execution follows the executor contract in [`../../README.md`](../../README.md#the-executor-contract), which owns the rule: one round per session, first `todo` round only, status transitions in `queue-rounds.yaml`, stop.

This plan adds no exceptions to it.

## Decisions & Constraints

- **Executor provenance:** `prex (EF 1.5)` — the profile these rounds were generated with. Provenance only; see [the contract](../../README.md#the-executor-contract).
- **Spawn-and-wait, not `exec`** — child supervision and the post-flight obligations in `docs/reference/process-runtime.md` require control to return after the child exits.
- **Child-binary resolution** (highest first): `$CLAUDE_SESSION_CHILD_BIN` → config entry (`child_bin`) → `PATH` search → optional bundled vendor path.
- **Recursion guard**: marker env `CLAUDE_SESSION_REENTRY=1` + `current_exe().canonicalize()` self-check so the wrapper never re-invokes itself as the child.
- **Exit status**: child code N → wrapper exits N unchanged; a signal-killed child is reproduced by re-raising the signal on the wrapper, with `128 + N` (clamped) as the fallback. Specified in `docs/reference/exit-codes.md`.
- **Signal handling is partial, not blanket.** The child shares the wrapper's foreground process group, so terminal-generated signals (`SIGINT`, `SIGQUIT`, `SIGTSTP`, `SIGCONT`, `SIGWINCH`) already reach it — forwarding those double-delivers. The wrapper forwards only `SIGTERM`, `SIGHUP`, `SIGUSR1`, and `SIGUSR2`, and re-raises `SIGSTOP` on itself for `SIGTSTP`. The matrix is in `docs/reference/process-runtime.md`; **implement it as written rather than reasoning it out afresh**.
- **Child env**: inherit parent env, REMOVE internal `CLAUDE_SESSION_*` keys (except the intentional `REENTRY` marker), SET the variables the table in `docs/reference/process-runtime.md` credits — `CLAUDE_CONFIG_DIR` points at the **account** configuration directory, which is what lets runs of one account share a saved login (`docs/decisions/ADR-0025-share-one-native-login-per-account.md`). The user's argv is forwarded verbatim as `OsString` as an untouched suffix.
- **The proxy seam is inheritance**: the wrapper snapshots the environment and removes only its own `CLAUDE_SESSION_` namespace, so a fronting proxy is pointed at by exporting `ANTHROPIC_BASE_URL`. Build no injection surface (rejected in ADR-0057), and no compression, rewriting, or routing inside the wrapper.

## Rejected Alternatives

- **`exec` as the default** — rejected; supervision and post-flight work must run after the child exits. Recorded in amended `docs/decisions/ADR-0004-spawn-and-wait-child-supervision.md`.
- **Any request-manipulating logic inside the wrapper** — rejected; provide the environment seam only.
- **Parsing the child's grammar to rewrite flags** — rejected; forward verbatim and claim only a denylist of wrapper-owned flags. Recorded in `docs/decisions/ADR-0002-verbatim-argv-passthrough.md`.

## Risks & Edge Cases

- Signal handling is platform-sensitive and Unix-only (`docs/reference/process-runtime.md` scopes the platform). Keep the implementation focused and async-signal-safe. **The double-delivery trap is the specific hazard**: blanket forwarding makes one Ctrl-C look like two to a child that counts them, and it will not show up without a test that counts deliveries.
- Env leakage: ensure no internal `CLAUDE_SESSION_*` value reaches the child except the intentional `REENTRY` marker.
- Non-UTF-8 argv: preserve `OsString` end-to-end.
- Child binary missing/non-executable: map to sysexits 127/126 with a four-part error + hint.

## Completion

When all rounds are done, set each round `done` in this plan's `queue-rounds.yaml` and set this plan `done` in the top-level `.implementation-plans/queue-plans.yaml`. Nothing moves on disk.
