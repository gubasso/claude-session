# Docs & Hardening R2: Comprehensive, Graceful `doctor`

> Plan: cs-docs-hardening | Round: 2 of 4 | Complexity: L | Executor: prex (EF 1.5) | Generated: 2026-06-19 |
> Repo: /workspaces/claude-session

## Context

`doctor` is the user's self-diagnostic. The shell-tool lesson is that it must report every subsystem's
health gracefully and never abort the whole check on one bad subsystem (their `doctor` had to
special-case profile resolution to avoid aborting). This round hardens `doctor` across all subsystems
now that they exist: config layout, child binary + version floor, session dirs, accounts + seeds, auth,
and generated settings. `cs-foundation` provided the `doctor` stub; later plans added subsystem checks
incrementally — this round makes it comprehensive and uniform.

## Previous Rounds

`cs-foundation`: `doctor` stub. `cs-isolation`/`cs-wrapper-runtime`/`cs-accounts-auth`/
`cs-config-composition`: their subsystems + partial doctor hooks. Expect all to exist.

## Scope of This Round

- IN scope: a comprehensive `commands/doctor.rs` that checks, **each independently and without
  aborting**: config tree presence/validity; resolved child `claude` binary + minimum version floor;
  session-root resolution + secure-dir health; account registry + seed validity + current account;
  auth readiness (subscription seed or token fallback); generated `settings.json` freshness; with a
  three/four-part error + hint per failing check; `--json` structured output aggregating all results
  and an overall pass/warn/fail. A documented minimum supported `claude` version (baseline 2.1.183) as
  a checked floor.
- OUT of scope: completions/man pages (round 3); ADR/release (round 4).

## Current State

### Key Files

- `/workspaces/claude-session/src/commands/doctor.rs` — make comprehensive.
- `/workspaces/claude-session/src/services/*` — read-only health probes (config, session, account).
- `/workspaces/claude-session/src/adapters/spawner.rs` — `child_version_line` for the version floor.

### Existing Patterns

Reference (codex-session `doctor.rs` + the shell tool's `doctor` lesson, inspiration only): doctor must
run subsystem resolvers with error capture rather than aborting in the global resolver; report
structured results; a min-child-version check (codex used a `REQUIRED_..._VERSION` floor). Output
discipline: human table → stderr or stdout per convention; `--json` machine output → stdout.

## Implementation Steps

### First Step: Mark this round as started

In this plan's `queue-rounds.yaml`, set this round's (`item: doctor-hardening`) `status` to `doing`.

### Step 1: Subsystem probes

Add read-only health probes (config, child+version floor, session dirs, accounts+seeds+auth, generated
settings), each returning a result rather than aborting.

### Step 2: Aggregate + report

Aggregate into an overall pass/warn/fail; emit a human report and a `--json` structured output; each
failure carries a four-part message + hint.

### Step 3: Tests

`assert_cmd`-test `doctor`/`doctor --json` against fixtures with one broken subsystem (verify it
reports, does not abort, and exits non-zero appropriately).

### Final Step: Update the queue

1. In this plan's `queue-rounds.yaml`, set this round's (`item: doctor-hardening`) `status` to `done`.

## Acceptance Criteria

- [ ] `doctor` checks config/child+version/session/accounts/auth/generated-settings, each independently.
- [ ] One broken subsystem is reported (with a four-part message + hint) without aborting the others.
- [ ] `doctor --json` aggregates results with an overall pass/warn/fail and a correct exit code.
- [ ] The minimum `claude` version floor is checked and surfaced.
- [ ] `assert_cmd` tests pass.
- [ ] This plan's `queue-rounds.yaml` shows round `doctor-hardening` as `done`.

## Next Round

Round 3 (`completions-and-manpages`) finalizes `clap_complete` completions, the `version` verb, and
`clap_mangen` man pages against the final flag set.
