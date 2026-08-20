# 034 — Account usage reporting

## Goal

Report each account's live usage standing — the rate-limit windows the child's own Usage panel shows — from the wrapper, so an operator chooses an account without opening a session to ask.

## Appetite

2 implementation sessions.

## Core

`account usage` reports the provider's answer for every account that can ask, and names why one cannot.

## In scope

- One decision settling the carry: the usage endpoint is provider-owned rather than child-owned, the hardest case [ADR-0089](../../../decisions/ADR-0089-carry-a-child-owned-fact-only-against-an-obligation.md) has met, and the registry-boundary question Q-012 raises bounds what registering it means.
- The `account usage` subcommand, reporting per-account standing with `--json`, in the accounts namespace because the answer is a property of one account's credential.
- A perishable-fact entry in [research tracking](../../../reference/research-tracking.yaml) with a cadence and a re-verification recipe, because such endpoints are undocumented and their response shapes move.
- An honest report for a token account, whose grant the provider refuses `user:profile` and which therefore can never answer; the refusal is named, not omitted.

## Out of scope

- Quota-aware account selection, failover, or scoring; reporting is the whole surface.
- Caching, polling, or any daemon; a run asks once and prints.
- Modes the wrapper does not store, and any billing surface beyond the windows the report needs.

## Governed by

- [AGENTS.md](../../../../AGENTS.md)
- [Charter](../../charter.md)
- [ADR-0025](../../../decisions/ADR-0025-share-one-native-login-per-account.md)
- [ADR-0089](../../../decisions/ADR-0089-carry-a-child-owned-fact-only-against-an-obligation.md)
- [Accounts](../../../reference/accounts.md)
- [CLI surface](../../../reference/cli-surface.md)
- [Exit codes](../../../reference/exit-codes.md)
- [Logging and output](../../../reference/logging-and-output.md)
- [Research tracking](../../../reference/research-tracking.yaml)
- [Coding conventions](../../../reference/coding-conventions.md)
- [Testing and quality](../../../reference/testing-and-quality.md)
- [Development workflow](../../../guides/development-workflow.md)

## Acceptance

- When `account usage` runs against a saved login, it shall report that account's current windows as the provider answers them.
- When an account's mode cannot ask, the report shall name the refusal rather than omit the account.
- When the provider's answer cannot be parsed, the wrapper shall fail with a code and a hint naming the tracking entry, rather than report a guess.
- When the work lands, the endpoint shall be tracked as a perishable fact with a cadence.

## Rabbit holes

- Modelling the endpoint's whole schema; escape: parse the windows the report needs and refuse the rest honestly.
- Building selection logic on the report; escape: out of scope by name.
- Predicting the provider's shape changes; escape: the tracking entry owns freshness, the parser fails closed.

## Done when

The subcommand reports live standing for login accounts, names token mode's refusal, the carry decision is recorded, the endpoint is tracked, and `just hooks` is green.

## Revisions

None.
