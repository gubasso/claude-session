# Accounts & Auth R4: `account` CLI Verbs

> Plan: cs-accounts-auth | Round: 4 of 4 | Complexity: L | Executor: prex (EF 1.5) | Generated: 2026-06-19 | Repo: repository root

## Context

With discovery, both login modes, and launch resolution in place, `claude-session` exposes the user-facing `account` command surface — following the four-edit subcommand rule, with per-verb machine output and unconditional secret redaction. This is the final round of the accounts/auth plan. Rounds 1–3 built the machinery; this round wires it to the CLI and to the `doctor` catalog.

## Previous Rounds

This plan R1: account store, mode metadata, marker, resolver. R2: `account login` in both modes. R3: launch resolution, precedence warnings, version floor. `cs-foundation`: clap skeleton + `doctor` stub + `Ui`. Expect all to exist.

## Scope of This Round

- IN scope: the `account` subcommand tree per the four-edit rule — `cli/account.rs` (clap `<Verb>Args`), `cli.rs` (register the `Account` variant, replacing the reserved stub), `commands/account.rs` (free `run` handlers), the `commands/dispatch.rs` match arm; verb-level `--json` output via `Ui` with **secret redaction**; `--yes` on `remove` and its confirmation; adding the two account entries to the `doctor` catalog. Tests via `assert_cmd`.
- OUT of scope: new auth machinery (rounds 1–3); config composition (`cs-config-composition`); completions, man pages, and `version` (`cs-docs-hardening`).

## Current State

### Key Files

- `src/cli/account.rs` — new (clap shapes; the subcommand tree behind the `--account` selection flag).
- `src/cli.rs` — register the `Account` `Commands` variant.
- `src/commands/account.rs` — handlers.
- `src/commands/doctor.rs` — add the catalog's account entries.

### Existing Patterns

The four-edit rule — `cli/<verb>.rs`, the `cli.rs` enum variant, `commands/<verb>.rs` with a free `run`, and the dispatch arm — is specified in `docs/explanation/architecture.md`. The `account` verb's place in the wrapper's grammar is in `docs/reference/cli-surface.md`; the per-subcommand contract — the exact subcommand set, each one's arguments, what each reports, and every failure mode — is the table in `docs/reference/accounts.md`, which this round implements rather than restates.

`--json` is **verb-level**, never global — `docs/decisions/ADR-0024-machine-output-is-a-per-verb-flag.md` — and each verb owns its own document shape per `docs/decisions/ADR-0032-give-each-verb-its-own-json-document.md`. Output discipline is specified in `docs/reference/logging-and-output.md`: results to standard output through the single writer, diagnostics and prompts to standard error, and in JSON mode no human-oriented text appears on standard output at all.

**Credentials are never printed and never logged**, at any verbosity, in any format, with no escape hatch. `status` reports mode, recorded time, age, estimated expiry, the wrapper-owned token's `sha256[..8]` fingerprint, the child's own status probe, selection provenance, and shadowing — and never the token, a token prefix, or any child-credential content or fingerprint.

Exit behaviour follows the inspection-versus-assertion split in `docs/reference/exit-codes.md`: `list` and `status` are inspection verbs and exit `0` whatever they find, including `list` with no accounts and `status` reporting unusable authentication. Only a wrapper-side failure makes them non-zero.

Two catalog entries cover this subsystem, and their ids are public API specified in `docs/reference/logging-and-output.md`. Both are soft and skipped when no account exists. Adding a private check to `doctor` instead of a catalog entry is what `docs/decisions/ADR-0018-one-probe-set-with-stable-check-ids.md` forbids.

Removal confirms unless `--yes` is present, warns first when the account is selected, and reports plainly that deleting local state is not upstream revocation. Declining exits `0` and says nothing was removed.

## Implementation Steps

### First Step: Mark this round as started

In this plan's `queue-rounds.yaml`, set this round's (`item: account-commands`) `status` to `doing`.

### Step 1: clap shapes

Add `cli/account.rs` with an arg struct per subcommand in the `docs/reference/accounts.md` table, each carrying its own `--json`; register the `Account` variant in `cli.rs`.

### Step 2: Handlers

In `commands/account.rs`, implement the free `run` handlers over rounds 1–3's services. Emit each verb's document through `Ui` with redaction applied at the type level rather than at the print site.

### Step 3: doctor catalog

Extend `commands/doctor.rs` with the catalog's two account entries at the id, scope, severity, and `err.kind` `docs/reference/logging-and-output.md` gives them. Both skip when no account exists, and a skip never affects the exit code.

### Step 4: Dispatch + tests

Add the `commands/dispatch.rs` match arm. With `assert_cmd`, test each verb's JSON document, that `list` and `status` exit `0` on an empty or unusable state, the `remove` confirmation and `--yes`, and that no output stream ever carries token material.

### Final Step: Update the queue

1. In this plan's `queue-rounds.yaml`, set this round's (`item: account-commands`) `status` to `done`.
2. All rounds are now done, so in the top-level `.implementation-plans/queue-plans.yaml` set this plan's (`item: cs-accounts-auth`) `status` to `done`. Leave the plan directory in place.

## Acceptance Criteria

- [ ] Every subcommand in the `docs/reference/accounts.md` table exists via the four-edit rule, and no subcommand outside it does.
- [ ] Each verb's `--json` is verb-level and machine-parseable; no credential appears in any output at any verbosity.
- [ ] `list` with no accounts and `status` on unusable authentication both exit `0`.
- [ ] `doctor` gains the catalog's two account entries by id, and both skip rather than fail when no account exists.
- [ ] Output discipline holds: data to standard output, diagnostics and prompts to standard error.
- [ ] This plan's `queue-rounds.yaml` shows round `account-commands` as `done` and the top-level `queue-plans.yaml` shows `cs-accounts-auth` as `done`.

## Next Round

This is the final round of this plan. `cs-config-composition` (parallel sibling) generates the settings document that lands in the composed-settings store; `cs-docs-hardening` finalizes `doctor`, completions, man pages, docs, and release.
