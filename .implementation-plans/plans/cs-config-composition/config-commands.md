# Config Composition R4: `config` & `profile` CLI Verbs

> Plan: cs-config-composition | Round: 4 of 4 | Complexity: L | Executor: prex (EF 1.5) | Generated: 2026-06-19 | Repo: repository root

## Context

With composition implemented, `claude-session` exposes the user-facing `config` and `profile` verbs to inspect paths, compose/preview, validate, and report status + provenance — following the four-edit rule with `--json`. This is the final round of the config-composition plan. Rounds 1–3 built the layout, merge engine, and generation; this round wires them to the CLI and replaces the foundation's `config` stub.

## Previous Rounds

This plan R1: layout + models. R2: merge engine + provenance. R3: generation + freshness + sidecar + trust state. `cs-foundation`: clap skeleton + `config` stub + `Ui`. Expect all to exist.

## Scope of This Round

- IN scope: the `config` subcommand (`view`, `path`, `compose`, `validate`, `status`) and a `profile` subcommand (`list`, `status`) per the four-edit rule (`cli/config.rs` + `cli/profile.rs`, `cli.rs` enum variants replacing the `config` stub, `commands/config.rs` + `commands/profile.rs` free `run` handlers, the `commands/dispatch.rs` match arms); `--json` output via `Ui`; `compose`/`validate` run the engine in preview/check mode and surface **provenance** + unknown-key warnings; `status` shows the active profile, resolved pieces, and whether the generated `settings.json` is fresh; wire config health into `doctor`.
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

Add `cli/config.rs` (`view|path|compose|validate|status`) and `cli/profile.rs` (`list|status`); register the variants in `cli.rs` (replacing the `config` stub).

### Step 2: Handlers

Implement `commands/config.rs` + `commands/profile.rs` free `run` handlers calling the layout/merge/generate services; emit `--json` via `Ui`; surface provenance + unknown-key warnings.

### Step 3: doctor integration

Extend `doctor` to report config layout, active profile, and generated-settings freshness gracefully.

### Step 4: Dispatch + tests

Add the `commands/dispatch.rs` match arms; `assert_cmd`-test `config compose`/`status`/`profile list` output (including provenance).

### Final Step: Update the queue

1. In this plan's `queue-rounds.yaml`, set this round's (`item: config-commands`) `status` to `done`.
2. All rounds are now done, so in the top-level `.implementation-plans/queue-plans.yaml` set this plan's (`item: cs-config-composition`) `status` to `done`. Leave the plan directory in place.

## Acceptance Criteria

- [ ] Every `config` and `profile` subcommand in the table in `docs/reference/configuration.md` works via the four-edit rule with verb-level `--json`, and no subcommand outside it exists.
- [ ] `compose`/`validate` surface per-key provenance and unknown-key warnings.
- [ ] `status` reports the active profile, resolved pieces, and freshness.
- [ ] `doctor` reports config health without aborting.
- [ ] `assert_cmd` tests pass.
- [ ] This plan's `queue-rounds.yaml` shows round `config-commands` as `done` and the top-level `queue-plans.yaml` shows `cs-config-composition` as `done`.

## Next Round

This is the final round of this plan. `cs-docs-hardening` documents the headroom seam, hardens `doctor`, finalizes completions/version, writes the prior-art docs, and closes out ADRs + release.
