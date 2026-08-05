# Docs & Hardening R2: Comprehensive, Graceful `doctor`

> Plan: cs-docs-hardening | Round: 2 of 4 | Complexity: L | Executor: prex (EF 1.5) | Generated: 2026-06-19 | Repo: repository root

## Context

`doctor` is the user's self-diagnostic, and it must report every subsystem's health without one bad subsystem aborting the rest. This round hardens it across every subsystem now that they exist, against the catalog in `docs/reference/doctor.md`. `cs-foundation` provided the `doctor` stub; later plans added their own catalog entries incrementally — this round makes the whole set complete and uniform.

## Previous Rounds

`cs-foundation`: `doctor` stub. `cs-isolation`/`cs-wrapper-runtime`/`cs-accounts-auth`/ `cs-config-composition`: their subsystems + partial doctor hooks. Expect all to exist.

## Scope of This Round

- IN scope: a comprehensive `commands/doctor.rs` running **every** entry in the catalog table in `docs/reference/doctor.md`, each independently and without aborting; the full verb grammar that page specifies — `--json`, `--list`, and `--strict`, all verb-level and combinable; the report shape and the JSON document that page specifies; the exit rule it specifies, including the bare `1` that `--strict` produces.
- OUT of scope: completions/man pages (round 3); ADR/release (round 4). The catalog's contents, ids, severities, and the checked version floor are **not** decided here — that table owns them.

## Current State

### Key Files

- `src/commands/doctor.rs` — make comprehensive.
- `src/services/*` — read-only health probes (config, session, account).
- `src/adapters/spawner.rs` — `child_version_line` for the version floor.

### Existing Patterns

The full check catalog, its ids, scopes, hard-versus-soft classification, the `err.kind` each failure exits with, the remediation templates, the report shape, and the JSON document are all specified in `docs/reference/doctor.md`. Implement that page; do not restate it here and do not add a check the catalog does not list. There is exactly **one** probe set, and a command guard reads the same one — `docs/decisions/ADR-0018-one-probe-set-with-stable-check-ids.md`. The stream each byte leaves on remains `docs/reference/logging-and-output.md`.

The child's own health output is folded into the report rather than replacing it, under `docs/decisions/ADR-0045-compose-doctor-with-the-child-report.md`.

Three rules govern the design. **Every check runs independently and one failure never aborts the rest** — a `doctor` that stops at the first problem is useless exactly when it is needed, because the first problem is often a consequence of the third. **An inert soft check reports `skipped` with a reason and never gates**, since failing `doctor` over a feature the user has not configured punishes them for not using it, and a skip never touches the exit code. And **check ids are public API**: scripts match them, so the table grows by appending and a rename is a breaking change.

Exit is `0` when no hard check fails, otherwise the `err.kind` code of the first failing hard check **in catalog order** — which is why that table's order is contractual. `--strict` adds one rule and nothing else, specified in `docs/decisions/ADR-0034-exit-one-when-doctor-strict-promotes-a-warning.md`. Each failure carries the four-part error shape from `docs/reference/exit-codes.md`.

A below-floor child version is a `doctor` **warning** and a `login`-mode launch **failure**: the same fact at two severities, because only one of them is a precondition. The floor itself is a perishable, externally-owned fact tracked in `docs/reference/research-tracking.yaml` and named in `docs/decisions/ADR-0031-enforce-the-child-refresh-lock-version-floor.md`; an unparsable version string is reported here, not fatal.

## Implementation Steps

### First Step: Mark this round as started

In this plan's `queue-rounds.yaml`, set this round's (`item: doctor-hardening`) `status` to `doing`.

### Step 1: Complete the probe set

Add a read-only probe for every catalog entry not yet implemented by an earlier plan, each returning a result rather than aborting, keyed by the id the catalog gives it.

### Step 2: Report and machine output

Emit the human report and the `--json` document exactly as `docs/reference/doctor.md` specifies them, including the bracketed status words, the indented hint line, the summary line, and the document's field-omission rules.

### Step 3: Grammar and exit

Wire `--list` (print the catalog without running anything) and `--strict`. Implement the exit rule: `0` with no hard failure, the first failing hard check's code otherwise, and `1` when `--strict` promotes a warning.

### Step 4: Tests

`assert_cmd`-test `doctor` in text, `--json`, and `--list` modes against fixtures with one broken subsystem: verify it reports, does not abort, and exits the code the catalog order predicts. Test that `--strict` promotes a warning to `1` and can never fail a passing catalog, and that a skip never changes the exit.

### Final Step: Update the queue

1. In this plan's `queue-rounds.yaml`, set this round's (`item: doctor-hardening`) `status` to `done`.

## Acceptance Criteria

- [ ] Every catalog entry in `docs/reference/doctor.md` runs, by its documented id, and no check outside that table exists.
- [ ] One broken subsystem is reported (with a four-part message + hint) without aborting the others.
- [ ] The `--json` document matches the specified shape, and the report matches the specified text form.
- [ ] `--list` prints the catalog without running it; `--strict` promotes a warning to `1` and nothing else.
- [ ] The exit is `0`, the first failing hard check's code, or the bare `1`, per the specified rule.
- [ ] `assert_cmd` tests pass.
- [ ] This plan's `queue-rounds.yaml` shows round `doctor-hardening` as `done`.

## Next Round

Round 3 (`completions-and-manpages`) finalizes `clap_complete` completions, the `version` verb, and `clap_mangen` man pages against the final flag set.
