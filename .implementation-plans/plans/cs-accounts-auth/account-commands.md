# Accounts & Auth R4: `account` CLI Verbs

> Plan: cs-accounts-auth | Round: 4 of 4 | Complexity: L | Executor: prex (EF 1.5) | Generated: 2026-06-19 | Repo: repository root

## Context

With the registry, managed login, resolver, and auth gate in place, `claude-session` exposes the user-facing `account` command surface to add, list, inspect, remove, and refresh accounts — following the four-edit subcommand rule, with `--json` machine output and secret redaction. This is the final round of the accounts/auth plan. Rounds 1–3 built the machinery; this round wires it to the CLI and to `doctor`.

## Previous Rounds

This plan R1: `Registry`. R2: managed login + hardened I/O. R3: resolver, gate, session copy, fallback, trust sync-back. `cs-foundation`: clap skeleton + `doctor` stub + `Ui` (stdout/stderr + `--json`). Expect all to exist.

## Scope of This Round

- IN scope: the `account` subcommand tree per the four-edit rule — `cli/account.rs` (clap `<Verb>Args`), `cli.rs` (register the `Account` variant — replacing the reserved stub), `commands/account.rs` (free `run` handlers for `add|list|status|remove|refresh`), the `commands/dispatch.rs` match arm; `--json` output via `Ui` with **secret redaction** (never print tokens; show `has_auth`/`last_used_at` instead); wiring account health into `doctor` (accounts present, seeds valid, current account). Tests via `assert_cmd`.
- OUT of scope: new auth machinery (done in R1–R3); config composition (`cs-config-composition`); completions/version (`cs-docs-hardening`).

## Current State

### Key Files

- `src/cli/account.rs` — new (clap shapes; replaces the reserved `--account` stub semantics with the full subcommand tree).
- `src/cli.rs` — register the `Account` `Commands` variant.
- `src/commands/account.rs` — handlers.
- `src/commands/doctor.rs` — add account checks.

### Existing Patterns

The four-edit rule — `cli/<verb>.rs`, the `cli.rs` enum variant, `commands/<verb>.rs` with a free `run`, and the dispatch arm — is specified in `docs/explanation/architecture.md`. The `account` verb and its place in the wrapper's grammar are in `docs/reference/cli-surface.md`; the per-subcommand contract — arguments, what each reports, and every failure mode — is in `docs/reference/accounts.md`.

Output discipline is specified in `docs/reference/logging-and-output.md`: results to stdout through the single writer, diagnostics to stderr, and `--json` is a **mode** — in JSON mode no human-oriented text appears on stdout at all.

**Credentials, tokens, and API keys are never printed and never logged**, at any verbosity, with no verbose escape hatch. Report presence and last-used instead of contents; that is all a user needs to answer "is this account usable?".

## Implementation Steps

### First Step: Mark this round as started

In this plan's `queue-rounds.yaml`, set this round's (`item: account-commands`) `status` to `doing`.

### Step 1: clap shapes

Add `cli/account.rs` with the `account add|list|status|remove|refresh` arg structs; register the `Account` variant in `cli.rs`.

### Step 2: Handlers

In `commands/account.rs`, implement the free `run` handlers calling the registry/login/resolver; emit `--json` via `Ui` with secret redaction.

### Step 3: doctor integration

Extend `commands/doctor.rs` to report accounts present, seed validity, and the current account, with graceful (non-aborting) error reporting.

### Step 4: Dispatch + tests

Add the `commands/dispatch.rs` match arm; `assert_cmd`-test `account list`/`status` JSON output and redaction.

### Final Step: Update the queue

1. In this plan's `queue-rounds.yaml`, set this round's (`item: account-commands`) `status` to `done`.
2. All rounds are now done, so in the top-level `.implementation-plans/queue-plans.yaml` set this plan's (`item: cs-accounts-auth`) `status` to `done`. Leave the plan directory in place.

## Acceptance Criteria

- [ ] `account add|list|status|remove|refresh` work via the four-edit rule; `--json` output is machine-parseable and redacts secrets.
- [ ] `doctor` reports account/seed/current-account health without aborting on a bad account.
- [ ] Output discipline holds (data → stdout, diagnostics → stderr; no credentials printed).
- [ ] `assert_cmd` tests pass.
- [ ] This plan's `queue-rounds.yaml` shows round `account-commands` as `done` and the top-level `queue-plans.yaml` shows `cs-accounts-auth` as `done`.

## Next Round

This is the final round of this plan. `cs-config-composition` (parallel sibling) generates the `settings.json` that lands in the isolated session dir; `cs-docs-hardening` finalizes doctor, completions, docs, and release.
