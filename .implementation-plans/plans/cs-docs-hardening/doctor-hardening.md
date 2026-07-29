# Docs & Hardening R2: Comprehensive, Graceful `doctor`

> Plan: cs-docs-hardening | Round: 2 of 4 | Complexity: L | Executor: prex (EF 1.5) | Generated: 2026-06-19 | Repo: repository root

## Context

`doctor` is the user's self-diagnostic. The shell-tool lesson is that it must report every subsystem's health gracefully and never abort the whole check on one bad subsystem (their `doctor` had to special-case profile resolution to avoid aborting). This round hardens `doctor` across all subsystems now that they exist: config layout, child binary + version floor, session dirs, accounts + seeds, auth, and generated settings. `cs-foundation` provided the `doctor` stub; later plans added subsystem checks incrementally — this round makes it comprehensive and uniform.

## Previous Rounds

`cs-foundation`: `doctor` stub. `cs-isolation`/`cs-wrapper-runtime`/`cs-accounts-auth`/ `cs-config-composition`: their subsystems + partial doctor hooks. Expect all to exist.

## Scope of This Round

- IN scope: a comprehensive `commands/doctor.rs` that checks, **each independently and without aborting**: config tree presence/validity; resolved child `claude` binary + minimum version floor; session-root resolution + secure-dir health; account registry + seed validity + current account; auth readiness (subscription seed or token fallback); generated `settings.json` freshness; with a three/four-part error + hint per failing check; `--format json` structured output aggregating all results and an overall pass/warn/fail. A documented minimum supported `claude` version (baseline 2.1.183) as a checked floor.
- OUT of scope: completions/man pages (round 3); ADR/release (round 4).

## Current State

### Key Files

- `src/commands/doctor.rs` — make comprehensive.
- `src/services/*` — read-only health probes (config, session, account).
- `src/adapters/spawner.rs` — `child_version_line` for the version floor.

### Existing Patterns

The full check catalog, its hard-versus-soft classification, and the output contract are specified in `docs/reference/logging-and-output.md`. Implement that table.

Two rules govern the design. **Every check runs independently and one failure never aborts the rest** — a `doctor` that stops at the first problem is useless exactly when it is needed, because the first problem is often a consequence of the third. And **an inert soft check never gates**: a feature the user has not configured reports as not-applicable, since failing `doctor` over unused features punishes the user for not using them.

The exit code is non-zero only when a **hard** check fails. Each failure carries the four-part error shape from `docs/reference/exit-codes.md`. The child version floor is a perishable fact tracked in `docs/reference/research-tracking.yaml`, and the check is defensive: an unparsable version string is reported, not fatal.

## Implementation Steps

### First Step: Mark this round as started

In this plan's `queue-rounds.yaml`, set this round's (`item: doctor-hardening`) `status` to `doing`.

### Step 1: Subsystem probes

Add read-only health probes (config, child+version floor, session dirs, accounts+seeds+auth, generated settings), each returning a result rather than aborting.

### Step 2: Aggregate + report

Aggregate into an overall pass/warn/fail; emit a human report and a `--format json` structured output; each failure carries a four-part message + hint.

### Step 3: Tests

`assert_cmd`-test `doctor` in both text and `--format json` modes against fixtures with one broken subsystem (verify it reports, does not abort, and exits non-zero appropriately).

### Final Step: Update the queue

1. In this plan's `queue-rounds.yaml`, set this round's (`item: doctor-hardening`) `status` to `done`.

## Acceptance Criteria

- [ ] `doctor` checks config/child+version/session/accounts/auth/generated-settings, each independently.
- [ ] One broken subsystem is reported (with a four-part message + hint) without aborting the others.
- [ ] `doctor` in `--format json` mode aggregates results with an overall pass/warn/fail and a correct exit code.
- [ ] The minimum `claude` version floor is checked and surfaced.
- [ ] `assert_cmd` tests pass.
- [ ] This plan's `queue-rounds.yaml` shows round `doctor-hardening` as `done`.

## Next Round

Round 3 (`completions-and-manpages`) finalizes `clap_complete` completions, the `version` verb, and `clap_mangen` man pages against the final flag set.
