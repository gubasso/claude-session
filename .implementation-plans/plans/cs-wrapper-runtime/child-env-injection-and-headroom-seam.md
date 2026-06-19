# Wrapper Runtime R3: Isolated Child Env & headroom/proxy Seam

> Plan: cs-wrapper-runtime | Round: 3 of 3 | Complexity: L | Executor: prex (EF 1.5) | Generated: 2026-06-19 |
> Repo: /workspaces/claude-session

## Context

`claude-session` isolates the child `claude` by setting `CLAUDE_CONFIG_DIR` to the per-account,
per-group session dir and scrubbing its own internal env namespace so the child sees a clean
environment. It must also let an external proxy (e.g. headroom) front `claude` by making the child env
— notably `ANTHROPIC_BASE_URL` — composeable/injectable, WITHOUT implementing any compression
internally. This round builds the typed child invocation/env, wires the isolation plan's resolved
session dir into the spawn, exposes the proxy seam, and adds end-to-end passthrough integration tests.
After this round, `claude-session <native args>` runs the real `claude` against an isolated config dir
keyed on the controlling terminal, optionally fronted by a proxy.

## Previous Rounds

This plan round 1: `Spawner` trait, child resolution, recursion guard. Round 2: `spawn_and_wait`,
signal forwarding, exit-code mapping. `cs-isolation`: `AppContext` exposes the resolved
per-account/per-group session dir. Expect all to exist and compile.

## Scope of This Round

- IN scope: `domain/child_invocation.rs` (`ChildInvocation { binary, args, env }` and
  `ChildEnv { inherit, remove, set }` with `scrubbed_default()` — inherit parent env, REMOVE all
  internal `CLAUDE_SESSION_*` keys, SET `CLAUDE_CONFIG_DIR=<session-dir>` + `CLAUDE_SESSION_REENTRY=1`);
  wiring `commands/pass_through.rs` to build the `ChildInvocation` from the `AppContext`-resolved
  session dir (replacing the round-2 pass-through env) and forward argv verbatim as `OsString`; an
  injectable proxy/headroom seam (compose `ANTHROPIC_BASE_URL` and arbitrary additional child env from
  config/CLI/env passthrough) with NO internal compression; end-to-end `assert_cmd` integration tests
  proving isolation (the stub child observes `CLAUDE_CONFIG_DIR`) and passthrough fidelity.
- OUT of scope: credential seeding into the session dir (`cs-accounts-auth`); composing `settings.json`
  (`cs-config-composition`); documenting the headroom integration as a guide/ADR (`cs-docs-hardening`).

## Current State

### Key Files

- `/workspaces/claude-session/src/domain.rs` (+ `src/domain/`) — add `child_invocation.rs`.
- `/workspaces/claude-session/src/commands/pass_through.rs` — build the isolated `ChildInvocation`.
- `/workspaces/claude-session/src/adapters/spawner.rs` — `spawn_and_wait` consumes `ChildInvocation`.
- `/workspaces/claude-session/src/context.rs` — provides the resolved session dir.

### Existing Patterns

Reference `child_invocation.rs` (codex-session, inspiration only): `ChildEnv::scrubbed_default()`
inherits parent env, removes all internal `CODEX_SESSION_*` keys, sets `CODEX_HOME=<session dir>` +
`CODEX_SESSION_REENTRY=1`. Analog: remove `CLAUDE_SESSION_*`, set `CLAUDE_CONFIG_DIR=<session dir>` +
`CLAUDE_SESSION_REENTRY=1`. headroom integration facts (brief §9): headroom is a token-compression
proxy; the clean seam is `ANTHROPIC_BASE_URL=http://localhost:<port>` pointing at `headroom proxy`
injected into the isolated child env — claude-session implements NO compression, only the env seam.
Confirmed native auth/proxy env vars: `ANTHROPIC_BASE_URL`, `ANTHROPIC_AUTH_TOKEN`, `ANTHROPIC_API_KEY`.

## Implementation Steps

### First Step: Mark this round as started

In this plan's `queue-rounds.yaml`, set this round's (`item: child-env-injection-and-headroom-seam`)
`status` to `doing`.

### Step 1: ChildInvocation/ChildEnv

In `domain/child_invocation.rs`, define `ChildInvocation` and `ChildEnv` with `scrubbed_default()`
(inherit, remove `CLAUDE_SESSION_*`, set `CLAUDE_CONFIG_DIR` + `REENTRY`).

### Step 2: Wire into pass-through

In `commands/pass_through.rs`, build the `ChildInvocation` from the resolved session dir + forwarded
argv (`OsString`), and hand it to `spawn_and_wait`.

### Step 3: Proxy/headroom seam

Allow composing `ANTHROPIC_BASE_URL` (and arbitrary additional child env) from config/CLI/env so an
external proxy can front `claude`. No internal compression. Document the seam in code comments
(referencing the `cs-docs-hardening` guide).

### Step 4: End-to-end tests

Add `assert_cmd` integration tests with a stub `claude` asserting it observes `CLAUDE_CONFIG_DIR` set
to the per-group dir, internal `CLAUDE_SESSION_*` scrubbed, argv preserved, and `ANTHROPIC_BASE_URL`
injected when configured.

### Final Step: Update the queue

1. In this plan's `queue-rounds.yaml`, set this round's (`item: child-env-injection-and-headroom-seam`)
   `status` to `done`.
2. All rounds are now done, so in the top-level `.implementation-plans/queue-plans.yaml` set this
   plan's (`item: cs-wrapper-runtime`) `status` to `done`. Leave the plan directory in place.

## Acceptance Criteria

- [ ] The spawned child sees `CLAUDE_CONFIG_DIR=<session-dir>` and no internal `CLAUDE_SESSION_*`
      values except the `REENTRY` marker.
- [ ] argv is forwarded verbatim (`OsString`-preserving), verified against a stub child.
- [ ] `ANTHROPIC_BASE_URL` (and additional child env) can be injected from config/CLI/env; no internal
      compression exists.
- [ ] End-to-end passthrough + isolation integration tests pass.
- [ ] This plan's `queue-rounds.yaml` shows round `child-env-injection-and-headroom-seam` as `done`
      and the top-level `queue-plans.yaml` shows `cs-wrapper-runtime` as `done`.

## Next Round

This is the final round of this plan. `cs-accounts-auth` builds named accounts and managed
subscription login on top of the isolated spawn; `cs-config-composition` generates the `settings.json`
that lands in the isolated session dir.
