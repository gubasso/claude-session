# Wrapper Runtime R3: Isolated Child Env & Passthrough Fidelity

> Plan: cs-wrapper-runtime | Round: 3 of 3 | Complexity: L | Executor: prex (EF 1.5) | Generated: 2026-06-19 | Repo: repository root

## Context

`claude-session` points the child at its account's configuration directory through `CLAUDE_CONFIG_DIR`, scrubs its own internal env namespace so the child sees a clean environment, and hands the profile's composed settings through a wrapper-owned argv prefix rather than through that variable. An external proxy (e.g. headroom) fronts `claude` through `ANTHROPIC_BASE_URL`, which the wrapper delivers by **inheriting it untouched** — there is no injection mechanism to build ([ADR-0057](../../../docs/decisions/ADR-0057-build-the-child-environment-by-prefix-scrub-and-marker.md)). This round builds the typed child invocation/env and the argv-prefix seam, and adds end-to-end passthrough integration tests. After this round, `claude-session <native args>` runs the real `claude` against the resolved account configuration, optionally fronted by a proxy the user exported.

## Previous Rounds

This plan round 1: `Spawner` trait, child resolution, recursion guard. Round 2: `spawn_and_wait`, signal forwarding, exit-code mapping. `cs-isolation`: `AppContext` exposes the resolved account directory and composed-settings entry. Expect all to exist and compile.

## Scope of This Round

- IN scope: `domain/child_invocation.rs` (`ChildInvocation { binary, args, env }` and a `ChildEnv` implementing the ordered algorithm in `docs/reference/process-runtime.md` — snapshot, scrub the `CLAUDE_SESSION_` prefix, then set `CLAUDE_SESSION_REENTRY=1` and the account-scoped `CLAUDE_CONFIG_DIR`, in that order); the wrapper-owned **argv prefix** seam of `docs/decisions/ADR-0028-pass-composed-settings-with-the-native-flag.md`, built empty here and supplied by `cs-config-composition`, with the user's argv as an untouched suffix; wiring `commands/pass_through.rs` to build the `ChildInvocation` (replacing the round-2 pass-through env) and forward argv verbatim as `OsString`; the byte-recording stub child of `docs/reference/testing-and-quality.md`; end-to-end `assert_cmd` integration tests proving isolation (the stub observes `CLAUDE_CONFIG_DIR`) and passthrough fidelity.
- OUT of scope: resolving an account and its stored mode (`cs-accounts-auth`); composing the settings document the prefix will carry (`cs-config-composition`); documenting the headroom integration as a guide (`cs-docs-hardening`). **Any environment-injection surface — a `--env` flag or a configuration key — is out of scope permanently**, rejected in ADR-0057.

## Current State

### Key Files

- `src/domain.rs` (+ `src/domain/`) — add `child_invocation.rs`.
- `src/commands/pass_through.rs` — build the isolated `ChildInvocation`.
- `src/adapters/spawner.rs` — `spawn_and_wait` consumes `ChildInvocation`.
- `src/context.rs` — provides the resolved account directory and composed-settings entry.

### Existing Patterns

The child-environment contract is the ordered algorithm in `docs/reference/process-runtime.md`: snapshot the parent environment, **remove every key whose bytes begin `CLAUDE_SESSION_`** — wrapper inputs such as `CLAUDE_SESSION_CHILD_BIN` included, not only internals — then set `CLAUDE_SESSION_REENTRY=1` and, when an account is selected, `CLAUDE_CONFIG_DIR`. **The order is load-bearing**: setting the marker before the scrub deletes it. The marker is the one internal variable deliberately left in place, because it is half of the recursion guard.

**The proxy seam is inheritance, so there is nothing to build.** `ANTHROPIC_BASE_URL` reaches the child because step 1 of the algorithm snapshots it and nothing removes it; a user fronting `claude` exports it. Implement no composition, no injection surface, no compression, no request rewriting, and no routing — ADR-0057 rejects the surface and `docs/explanation/wrapper-model.md` says why that boundary erodes if it is not stated. `ANTHROPIC_BASE_URL` is tracked as a perishable fact in `docs/reference/research-tracking.yaml`.

Argv is forwarded verbatim as `OsString`, preserving order, bytes, count, and empty arguments; see `docs/reference/cli-surface.md`.

## Implementation Steps

### First Step: Mark this round as started

In this plan's `queue-rounds.yaml`, set this round's (`item: child-env-injection-and-headroom-seam`) `status` to `doing`.

### Step 1: ChildInvocation/ChildEnv

In `domain/child_invocation.rs`, define `ChildInvocation` and a `ChildEnv` that is a pure function over a snapshot — scrub by prefix, then set the marker and `CLAUDE_CONFIG_DIR` — plus the argv prefix/suffix split. Keys and values are `OsString`; the snapshot comes from `std::env::vars_os`, never `vars`.

### Step 2: Wire into pass-through

In `commands/pass_through.rs`, build the `ChildInvocation` from the resolved directories, the wrapper-owned prefix, and the forwarded argv suffix (`OsString`), and hand it to `spawn_and_wait`.

### Step 3: The recording stub

Build the stub child of `docs/reference/testing-and-quality.md`: it writes `argv`, `environ`, and `cwd` as NUL-separated raw bytes into a directory the test names. No text, no JSON, no lossy conversion — a stub that normalizes makes the golden argv test pass against a broken wrapper. Note the proxy boundary in a code comment citing ADR-0057, since it is the kind of boundary that erodes silently.

### Step 4: End-to-end tests

Add `assert_cmd` integration tests with the stub asserting it observes `CLAUDE_CONFIG_DIR` set to the account configuration directory, exactly one `CLAUDE_SESSION_*` key (the marker), a non-UTF-8 ambient variable arriving unchanged, and the golden argv legs of `docs/reference/testing-and-quality.md` — `argv[0]`, an empty argument, a non-UTF-8 argument, and the sentinel.

### Final Step: Update the queue

1. In this plan's `queue-rounds.yaml`, set this round's (`item: child-env-injection-and-headroom-seam`) `status` to `done`.
2. All rounds are now done, so in the top-level `.implementation-plans/queue-plans.yaml` set this plan's (`item: cs-wrapper-runtime`) `status` to `done`. Leave the plan directory in place.

## Acceptance Criteria

- [ ] The spawned child sees `CLAUDE_CONFIG_DIR` set to the account configuration directory and exactly one `CLAUDE_SESSION_*` key, the `REENTRY` marker.
- [ ] The argv prefix seam exists and the user's tokens remain an untouched suffix with order, bytes, count, and `--` preserved.
- [ ] argv is forwarded verbatim (`OsString`-preserving), verified against the byte-recording stub, `argv[0]` included.
- [ ] A non-UTF-8 ambient variable reaches the child unchanged, and no injection surface exists.
- [ ] End-to-end passthrough + isolation integration tests pass.
- [ ] This plan's `queue-rounds.yaml` shows round `child-env-injection-and-headroom-seam` as `done` and the top-level `queue-plans.yaml` shows `cs-wrapper-runtime` as `done`.

## Next Round

This is the final round of this plan. `cs-accounts-auth` builds named accounts and both login modes on top of this spawn; `cs-config-composition` generates the `settings.json` that the argv prefix carries.
