# Config Composition R4: `config` & `profile` CLI Verbs

> Plan: cs-config-composition | Round: 4 of 4 | Complexity: L | Executor: prex (EF 1.5) | Generated: 2026-06-19 | Repo: repository root

## Context

With composition implemented, `claude-session` exposes the user-facing `config` and `profile` verbs. `config` takes no subcommand: one invocation resolves, validates, and reports the whole picture. Both follow the four-edit rule with `--json` ([ADR-0049](../../../docs/decisions/ADR-0049-collapse-config-inspection-into-one-verb.md)). This is the final round of the config-composition plan. Rounds 1–3 built the layout, merge engine, and generation; this round wires them to the CLI and replaces the foundation's `config` stub.

## Previous Rounds

This plan R1: layout + models. R2: merge engine + provenance. R3: generation + freshness + sidecar + trust state. `cs-foundation`: clap skeleton + `config` stub + `Ui`. Expect all to exist.

## Scope of This Round

- IN scope: the bare `config` verb and `profile list` per the four-edit rule (`cli/config.rs` + `cli/profile.rs`, `cli.rs` enum variants replacing the `config` stub, `commands/config.rs` + `commands/profile.rs` free `run` handlers, the `commands/dispatch.rs` match arms); `--json` output via `Ui`; `config` runs the engine in preview mode and reports the resolved wrapper configuration with per-key **provenance**, the consulted files, the active profile with resolved pieces, generated-`settings.json` freshness, and unknown-key warnings; `config` exits on a structural or type defect per [exit codes](../../../docs/reference/exit-codes.md#inspection-verbs-and-assertion-verbs); wire the same config-scoped probe subset into `doctor`.
- OUT of scope: new composition machinery (done R1–R3); accounts (`cs-accounts-auth`); docs/headroom (`cs-docs-hardening`).

## Current State

### Key Files

- `src/cli/config.rs`, `src/cli/profile.rs` — new.
- `src/cli.rs` — register `Config`/`Profile` variants.
- `src/commands/config.rs`, `src/commands/profile.rs` — handlers.
- `src/commands/doctor.rs` — add config checks.

### Existing Patterns

The `config` and `profile` verbs and what each reports are specified in `docs/reference/configuration.md`; the wrapper grammar they live in is in `docs/reference/cli-surface.md`. The four-edit rule is in `docs/explanation/architecture.md`.

Output discipline is specified in `docs/reference/logging-and-output.md`: results to stdout through the single writer, diagnostics to stderr, `--json` a mode rather than a decoration.

Two asymmetries to preserve. **Provenance** is per-key and reports which piece set each leaf — that is the difference between "this setting is wrong" and "this setting is wrong because that piece overrode this one". And **validation is deliberately asymmetric**: unknown keys in the wrapper's own configuration are errors, while unknown keys in the child's settings are warnings with provenance, because the child's schema evolves independently and rejecting a valid new setting is worse than passing it through.

## Implementation Steps

### First Step: Mark this round as started

In this plan's `queue-rounds.yaml`, set this round's (`item: config-commands`) `status` to `doing`.

### Step 1: clap shapes

Add `cli/config.rs` (no subcommand; `--profile` and `--json`) and `cli/profile.rs` (`list`); register the variants in `cli.rs` (replacing the `config` stub).

### Step 2: Handlers

Implement `commands/config.rs` + `commands/profile.rs` free `run` handlers calling the layout/merge/generate services; emit `--json` via `Ui`; surface provenance + unknown-key warnings.

### Step 3: doctor integration

Extend `doctor` to run the config-scoped checks. Under [ADR-0018](../../../docs/decisions/ADR-0018-one-probe-set-with-stable-check-ids.md) these are one catalog: `config` runs the config-scoped subset and `doctor` runs it whole, so implement the probes once and call them from both. `doctor` never renders configuration.

### Step 4: Dispatch + tests

Add the `commands/dispatch.rs` match arms; `assert_cmd`-test `config` and `profile list` output (including provenance) and `config`'s exit code on a type conflict.

### Final Step: Update the queue

1. In this plan's `queue-rounds.yaml`, set this round's (`item: config-commands`) `status` to `done`.
2. All rounds are now done, so in the top-level `.implementation-plans/queue-plans.yaml` set this plan's (`item: cs-config-composition`) `status` to `done`. Leave the plan directory in place.

## Acceptance Criteria

- [ ] Every command in the table in `docs/reference/configuration.md` works via the four-edit rule with verb-level `--json`, and no subcommand outside it exists — in particular `config` accepts none.
- [ ] One `config` invocation reports the resolved configuration with per-key provenance, consulted files, active profile with resolved pieces, freshness, and unknown-key warnings.
- [ ] `config` exits `0` on warnings and with the mapped code on a structural or type defect.
- [ ] `doctor` and `config` share one probe implementation and report config health without aborting.
- [ ] `assert_cmd` tests pass.
- [ ] This plan's `queue-rounds.yaml` shows round `config-commands` as `done` and the top-level `queue-plans.yaml` shows `cs-config-composition` as `done`.

## Next Round

This is the final round of this plan. `cs-docs-hardening` documents the headroom seam, hardens `doctor`, finalizes completions/version, writes the prior-art docs, and closes out ADRs + release.
