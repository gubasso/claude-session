# Accounts & Auth R2: `account login` in Both Stored Modes

> Plan: cs-accounts-auth | Round: 2 of 4 | Complexity: L | Executor: prex (EF 1.5) | Generated: 2026-06-19 | Repo: repository root

## Context

`account login` is where an account's stored authentication mode is chosen. In **login mode** the wrapper launches the child's own login against the account's `config/` and never inspects the result — the child owns that credential for its whole life. In **token mode** the wrapper ingests a long-lived subscription token, stores it privately, and injects it at launch. This round implements both flows and the ordered rotation that protects the second. Round 1 built the account store and hardened I/O; `cs-wrapper-runtime` provides the spawner.

## Previous Rounds

This plan round 1: the account store, `auth-mode.json`, the last-used marker, hardened I/O, the resolver. `cs-wrapper-runtime`: `Spawner`/`spawn_and_wait`, child resolution, child env construction. Expect both to exist.

## Scope of This Round

- IN scope: `services/account/login.rs` — the login-mode path (resolve or create the account, launch the child's own `auth login` through the `Spawner` with `CLAUDE_CONFIG_DIR` set to the account `config/`, record the mode); the token-mode path (`--token`, `--stdin`, and the mint-time correction flag: run the child's token-minting command with inherited standard streams, then read one line from the controlling terminal with echo disabled, or read one line from standard input under `--stdin`); the ordered rotation — probe through the child's documented status command, then rename token and mode metadata in that order under the credential lock; idempotence and the failed-first-login cleanup.
- OUT of scope: the `account` CLI verb tree (round 4), launch-time environment and precedence (round 3).

## Current State

### Key Files

- `src/services/account/login.rs` — new.
- `src/services/account/store.rs` — write `auth-mode.json` and the token file.
- `src/services/auth.rs` — reuse the hardened I/O from round 1.
- `src/adapters/spawner.rs` — reuse to run the child's own commands.

### Existing Patterns

Both flows, the mode table they write, and the exact ingest rules are specified in `docs/reference/accounts.md`; the decisions behind them are `docs/decisions/ADR-0025-share-one-native-login-per-account.md`, `docs/decisions/ADR-0026-store-and-inject-a-long-lived-subscription-token.md`, `docs/decisions/ADR-0027-ingest-secrets-only-from-stdin-or-a-terminal.md`, and `docs/decisions/ADR-0030-use-account-login-for-wrapper-authentication.md`. File modes, ownership checks, and atomic-write rules are in `docs/reference/xdg-storage.md`.

**Login mode copies nothing.** One saved login lives in the account's `config/` and every run of that account shares it, which is what lets the child coordinate refresh across concurrent processes. The wrapper does not implement the browser flow, does not read the resulting credential, and does not fingerprint it.

Four constraints are absolute and none of them has a verbosity escape hatch. Token material is **never** accepted through argv, an environment variable, a wrapper file flag, or scraped child output. The wrapper **never** calls an OAuth token endpoint. The wrapper **never** writes or refreshes the child's own credential file. A fingerprint is `sha256[..8]` of the **wrapper-owned** token file only, never of anything the child owns.

Rotation is ordered because a half-replaced token is worse than an expired one: verify the candidate before writing anything, then rename `oauth-token` and `auth-mode.json` in that order under the credential lock, so the metadata rename is the commit and no crash point leaves an unusable account (`docs/decisions/ADR-0067-commit-a-token-rotation-with-the-metadata-rename.md`). There is no staging file.

**Where the child stores credentials is a perishable, externally-owned fact** — a file inside its configuration directory — tracked in `docs/reference/research-tracking.yaml`. Consume it defensively: an unexpected result is a reported failure with a hint, never a panic and never a silent success.

Neither mode falls back to the other, and the two are never mixed within one invocation.

## Implementation Steps

### First Step: Mark this round as started

In this plan's `queue-rounds.yaml`, set this round's (`item: account-login`) `status` to `doing`.

### Step 1: Login mode

In `services/account/login.rs`, resolve or create the account and its `config/`, then launch the child's own login through the `Spawner` with the configuration-directory variable pointed at that `config/`. On success record `mode: login` in `auth-mode.json`. Do not read what the child wrote.

### Step 2: Token ingest

Implement the token-mode ingest exactly as `docs/reference/accounts.md` specifies: without `--stdin`, run the child's token-minting command with inherited streams and then read one line from the controlling terminal with echo disabled; with `--stdin`, read one line from standard input and never prompt. Reject empty and multi-line input. Do not parse a prefix or infer a lifetime.

### Step 3: Ordered rotation

Probe the candidate through the child's documented status command, passing it in the child environment so nothing lands on disk before it is proven. Then take the credential lock and write `oauth-token` followed by `auth-mode.json`, each by the atomic sequence. A failure before the first rename leaves the previous pair intact; a crash between the two leaves the new token under stale metadata, which the fingerprint comparison detects and a later login repairs.

### Step 4: Idempotence

Make repeated logins safe: create the account on the first success, replace the mode on a later success, and remove an incomplete account when the first login fails. Support the mint-time correction flag for a token minted earlier than ingest.

### Step 5: Tests

Test login mode with a stubbed child that writes a credential file, asserting the wrapper never opens it. Test token mode against a stubbed minting and status command: verify `0600` on the token file, that rejection paths leave the previous state intact, and that no test ever sees token material on standard output, in a log record, or in argv.

### Final Step: Update the queue

1. In this plan's `queue-rounds.yaml`, set this round's (`item: account-login`) `status` to `done`.

## Acceptance Criteria

- [ ] Login mode launches the child's own login against the account `config/` and reads nothing back from it.
- [ ] Token ingest accepts a value only from a terminal with echo off or from standard input; argv, environment, file flag, and scraped output are all rejected.
- [ ] Rotation is transactional: a failed probe leaves the previous token and mode metadata unchanged.
- [ ] The token file is `0600`; the recorded fingerprint is of the wrapper-owned file only.
- [ ] No token value reaches standard output, a log record, or a process argument list at any verbosity.
- [ ] This plan's `queue-rounds.yaml` shows round `account-login` as `done`.

## Next Round

Round 3 (`launch-auth-and-precedence`) resolves the stored mode at launch, builds the child's authentication environment, reports ambient and shadowing precedence, and enforces the child version floor.
