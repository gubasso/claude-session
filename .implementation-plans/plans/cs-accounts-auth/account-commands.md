# Accounts & Auth R4: `account` CLI Verbs

> Plan: cs-accounts-auth | Round: 4 of 4 | Complexity: L | Executor: prex (EF 1.5) | Generated: 2026-06-19 | Repo: /workspaces/claude-session

## Context

With the registry, managed login, resolver, and auth gate in place, `claude-session` exposes the user-facing `account` command surface to add, list, inspect, remove, and refresh accounts — following the four-edit subcommand rule, with `--json` machine output and secret redaction. This is the final round of the accounts/auth plan. Rounds 1–3 built the machinery; this round wires it to the CLI and to `doctor`.

## Previous Rounds

This plan R1: `Registry`. R2: managed login + hardened I/O. R3: resolver, gate, session copy, fallback, trust sync-back. `cs-foundation`: clap skeleton + `doctor` stub + `Ui` (stdout/stderr + `--json`). Expect all to exist.

## Scope of This Round

- IN scope: the `account` subcommand tree per the four-edit rule — `cli/account.rs` (clap `<Verb>Args`), `cli/mod.rs` (register the `Account` variant — replacing the reserved stub), `commands/account.rs` (free `run` handlers for `add|list|current|remove|refresh`), `main.rs` dispatch arm; `--json` output via `Ui` with **secret redaction** (never print tokens; show `has_auth`/`last_used_at` instead); wiring account health into `doctor` (accounts present, seeds valid, current account). Tests via `assert_cmd`.
- OUT of scope: new auth machinery (done in R1–R3); config composition (`cs-config-composition`); completions/version (`cs-docs-hardening`).

## Current State

### Key Files

- `/workspaces/claude-session/src/cli/account.rs` — new (clap shapes; replaces the reserved `--account` stub semantics with the full subcommand tree).
- `/workspaces/claude-session/src/cli/mod.rs` — register the `Account` `Commands` variant.
- `/workspaces/claude-session/src/commands/account.rs` — handlers.
- `/workspaces/claude-session/src/commands/doctor.rs` — add account checks.

### Existing Patterns

Reference (codex-session `cli/account.rs` + `commands/account/*`, inspiration only): subcommand tree `add/list/current/remove/refresh`; redaction of secrets in `show`/`list` unless explicitly verbose; `--json` machine output. Four-edit rule (`rust/cli-spec/02-subcommand-pattern.md`): `cli/<verb>.rs`, `cli/mod.rs` enum, `commands/<verb>.rs` free `run`, `main.rs` arm. Output discipline: data → stdout (JSON via `Ui`), diagnostics → stderr; never print credentials.

## Implementation Steps

### First Step: Mark this round as started

In this plan's `queue-rounds.yaml`, set this round's (`item: account-commands`) `status` to `doing`.

### Step 1: clap shapes

Add `cli/account.rs` with the `account add|list|current|remove|refresh` arg structs; register the `Account` variant in `cli/mod.rs`.

### Step 2: Handlers

In `commands/account.rs`, implement the free `run` handlers calling the registry/login/resolver; emit `--json` via `Ui` with secret redaction.

### Step 3: doctor integration

Extend `commands/doctor.rs` to report accounts present, seed validity, and the current account, with graceful (non-aborting) error reporting.

### Step 4: Dispatch + tests

Add the `main.rs` dispatch arm; `assert_cmd`-test `account list`/`current` JSON output and redaction.

### Final Step: Update the queue

1. In this plan's `queue-rounds.yaml`, set this round's (`item: account-commands`) `status` to `done`.
2. All rounds are now done, so in the top-level `.implementation-plans/queue-plans.yaml` set this plan's (`item: cs-accounts-auth`) `status` to `done`. Leave the plan directory in place.

## Acceptance Criteria

- [ ] `account add|list|current|remove|refresh` work via the four-edit rule; `--json` output is machine-parseable and redacts secrets.
- [ ] `doctor` reports account/seed/current-account health without aborting on a bad account.
- [ ] Output discipline holds (data → stdout, diagnostics → stderr; no credentials printed).
- [ ] `assert_cmd` tests pass.
- [ ] This plan's `queue-rounds.yaml` shows round `account-commands` as `done` and the top-level `queue-plans.yaml` shows `cs-accounts-auth` as `done`.

## Next Round

This is the final round of this plan. `cs-config-composition` (parallel sibling) generates the `settings.json` that lands in the isolated session dir; `cs-docs-hardening` finalizes doctor, completions, docs, and release.
