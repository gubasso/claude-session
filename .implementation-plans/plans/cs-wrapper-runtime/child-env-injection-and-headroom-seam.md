# Wrapper Runtime R3: Isolated Child Env & headroom/proxy Seam

> Plan: cs-wrapper-runtime | Round: 3 of 3 | Complexity: L | Executor: prex (EF 1.5) | Generated: 2026-06-19 | Repo: repository root

## Context

`claude-session` points the child at its account's configuration directory through `CLAUDE_CONFIG_DIR`, scrubs its own internal env namespace so the child sees a clean environment, and hands the group's composed settings through a wrapper-owned argv prefix rather than through that variable. It must also let an external proxy (e.g. headroom) front `claude` by making the child env — notably `ANTHROPIC_BASE_URL` — composeable/injectable, WITHOUT implementing any compression internally. This round builds the typed child invocation/env, the argv-prefix seam, and the proxy seam, and adds end-to-end passthrough integration tests. After this round, `claude-session <native args>` runs the real `claude` against the resolved account configuration, optionally fronted by a proxy.

## Previous Rounds

This plan round 1: `Spawner` trait, child resolution, recursion guard. Round 2: `spawn_and_wait`, signal forwarding, exit-code mapping. `cs-isolation`: `AppContext` exposes the resolved per-account/per-group session dir. Expect all to exist and compile.

## Scope of This Round

- IN scope: `domain/child_invocation.rs` (`ChildInvocation { binary, args, env }` and `ChildEnv { inherit, remove, set }` with `scrubbed_default()` — inherit parent env, REMOVE all internal `CLAUDE_SESSION_*` keys, SET the variables the table in `docs/reference/process-runtime.md` credits, including the account-scoped `CLAUDE_CONFIG_DIR` and `CLAUDE_SESSION_REENTRY=1`); the wrapper-owned **argv prefix** seam of `docs/decisions/0028-pass-composed-settings-with-the-native-flag.md`, built empty here and supplied by `cs-config-composition`, with the user's argv as an untouched suffix; wiring `commands/pass_through.rs` to build the `ChildInvocation` (replacing the round-2 pass-through env) and forward argv verbatim as `OsString`; an injectable proxy/headroom seam (compose `ANTHROPIC_BASE_URL` and arbitrary additional child env from config/CLI/env passthrough) with NO internal compression; end-to-end `assert_cmd` integration tests proving isolation (the stub child observes `CLAUDE_CONFIG_DIR`) and passthrough fidelity.
- OUT of scope: resolving an account and its stored mode (`cs-accounts-auth`); composing the settings document the prefix will carry (`cs-config-composition`); documenting the headroom integration as a guide/ADR (`cs-docs-hardening`).

## Current State

### Key Files

- `src/domain.rs` (+ `src/domain/`) — add `child_invocation.rs`.
- `src/commands/pass_through.rs` — build the isolated `ChildInvocation`.
- `src/adapters/spawner.rs` — `spawn_and_wait` consumes `ChildInvocation`.
- `src/context.rs` — provides the resolved session dir.

### Existing Patterns

The child-environment contract is specified in `docs/reference/process-runtime.md`: inherit the parent environment, **remove every internal `CLAUDE_SESSION_*` key**, set `CLAUDE_CONFIG_DIR` to the resolved **account** configuration directory, and set `CLAUDE_SESSION_REENTRY=1` — the one internal variable deliberately left in place, because it is half of the recursion guard. Scrubbing matters for two reasons: the wrapper's internal state is not the child's business, and a nested invocation must not inherit stale values.

**The proxy seam is a general mechanism, not a proxy feature.** Composed injections are arbitrary key-value pairs drawn from configuration or the command line; `ANTHROPIC_BASE_URL` is simply the one an external proxy needs. Implement the composition, not the proxy: `claude-session` performs no compression, no request rewriting, and no routing of its own. See `docs/explanation/wrapper-model.md` for why that boundary erodes if it is not stated. `ANTHROPIC_BASE_URL` is tracked as a perishable fact in `docs/reference/research-tracking.yaml`.

Argv is forwarded verbatim as `OsString`, preserving order, bytes, count, and empty arguments; see `docs/reference/cli-surface.md`.

## Implementation Steps

### First Step: Mark this round as started

In this plan's `queue-rounds.yaml`, set this round's (`item: child-env-injection-and-headroom-seam`) `status` to `doing`.

### Step 1: ChildInvocation/ChildEnv

In `domain/child_invocation.rs`, define `ChildInvocation` and `ChildEnv` with `scrubbed_default()` (inherit, remove `CLAUDE_SESSION_*`, set `CLAUDE_CONFIG_DIR` + `REENTRY`), plus the argv prefix/suffix split.

### Step 2: Wire into pass-through

In `commands/pass_through.rs`, build the `ChildInvocation` from the resolved directories, the wrapper-owned prefix, and the forwarded argv suffix (`OsString`), and hand it to `spawn_and_wait`.

### Step 3: Proxy/headroom seam

Implement composition of arbitrary child environment keys from configuration and the command line; `ANTHROPIC_BASE_URL` is one instance of that mechanism, not a special case in the code. No internal compression, rewriting, or routing. Note the boundary in a code comment citing `docs/explanation/wrapper-model.md`, since it is the kind of boundary that erodes silently.

### Step 4: End-to-end tests

Add `assert_cmd` integration tests with a stub `claude` asserting it observes `CLAUDE_CONFIG_DIR` set to the account configuration directory, internal `CLAUDE_SESSION_*` scrubbed, argv preserved after any wrapper-owned prefix, and `ANTHROPIC_BASE_URL` injected when configured.

### Final Step: Update the queue

1. In this plan's `queue-rounds.yaml`, set this round's (`item: child-env-injection-and-headroom-seam`) `status` to `done`.
2. All rounds are now done, so in the top-level `.implementation-plans/queue-plans.yaml` set this plan's (`item: cs-wrapper-runtime`) `status` to `done`. Leave the plan directory in place.

## Acceptance Criteria

- [ ] The spawned child sees `CLAUDE_CONFIG_DIR` set to the account configuration directory and no internal `CLAUDE_SESSION_*` values except the `REENTRY` marker.
- [ ] The argv prefix seam exists and the user's tokens remain an untouched suffix with order, bytes, count, and `--` preserved.
- [ ] argv is forwarded verbatim (`OsString`-preserving), verified against a stub child.
- [ ] `ANTHROPIC_BASE_URL` (and additional child env) can be injected from config/CLI/env; no internal compression exists.
- [ ] End-to-end passthrough + isolation integration tests pass.
- [ ] This plan's `queue-rounds.yaml` shows round `child-env-injection-and-headroom-seam` as `done` and the top-level `queue-plans.yaml` shows `cs-wrapper-runtime` as `done`.

## Next Round

This is the final round of this plan. `cs-accounts-auth` builds named accounts and both login modes on top of this spawn; `cs-config-composition` generates the `settings.json` that the argv prefix carries.
