# claude-session — Accounts & Subscription Auth

> Complexity: L | Rounds: 4 | Generated: 2026-06-19 | Repo: repository root

## Problem Statement

`claude-session` must support multiple named accounts, each with its own durable native identity, while owning only what the child does not: selection, mode metadata, and any wrapper-stored token. An account is a directory under the state base; the child owns everything below that account's `config/`, including its saved login. This plan adds account discovery and selection, `account login` in both stored modes, launch-time authentication with its precedence warnings and version floor, and the `account` CLI verbs. The design is specified in `docs/reference/accounts.md`. Depends on `cs-wrapper-runtime` (login and launch both go through the spawner) and, transitively, `cs-isolation`/`cs-foundation`.

## Strategy

Four rounds. R1 builds the account store — directory discovery, `auth-mode.json`, the last-used marker, hardened file I/O — and the selection resolver. R2 implements `account login` in both modes: launching the child's own login into the account `config/`, and transactional ingest of a long-lived subscription token. R3 resolves the stored mode at launch, contributes the child's authentication environment, warns about ambient precedence and shadowing, and enforces the child version floor. R4 exposes the `account` verbs and adds the catalog's account checks to `doctor`.

## Rounds

1. `account-discovery.md` — account directories, mode metadata, last-used marker, hardened I/O, selection resolver.
2. `account-login.md` — `account login` in login mode and token mode, with transactional rotation.
3. `launch-auth-and-precedence.md` — launch-time mode resolution, child environment, precedence warnings, version floor.
4. `account-commands.md` — the `account` verb tree, machine output, redaction, doctor catalog entries.

## Execution Commands

Any executor following [the contract](../../README.md#the-executor-contract) can run these rounds. `/prex` is the one used to generate them, shown here as a worked example:

```bash
# Execute the next todo round (executor reads queue-rounds.yaml, runs the first todo round, then stops):
/prex -ar @.implementation-plans/plans/cs-accounts-auth/

# Or target a specific round file directly:
/prex -ar .implementation-plans/plans/cs-accounts-auth/account-discovery.md
```

## Execution Discipline

Execution follows the executor contract in [`../../README.md`](../../README.md#the-executor-contract), which owns the rule: one round per session, first `todo` round only, status transitions in `queue-rounds.yaml`, stop.

This plan adds no exceptions to it.

## Decisions & Constraints

- **Executor provenance:** `prex (EF 1.5)` — the profile these rounds were generated with. Provenance only; see [the contract](../../README.md#the-executor-contract).
- **Two stored modes, chosen once by `account login`** and resolved deterministically afterwards. Ambient state never rewrites a stored mode. Specified in `docs/reference/accounts.md`.
- **One saved login per account, shared by its runs** — never copied per session, because the child's cross-process refresh coordination only protects processes sharing one file. `docs/decisions/0025-share-one-native-login-per-account.md`, which supersedes the earlier seed-and-copy model in `docs/decisions/0011-isolate-credentials-by-seed-and-session.md`.
- **The ownership boundary is absolute.** The wrapper owns selection, mode metadata, and any stored token; the child owns everything below the account `config/`. The wrapper never reads, copies, writes, refreshes, synchronizes, or fingerprints a child credential.
- **Secrets enter from a terminal or standard input only** — never argv, environment, a file flag, or scraped child output — and the wrapper never calls an OAuth endpoint. `docs/decisions/0027-ingest-secrets-only-from-stdin-or-a-terminal.md`.
- **No subcommand ever prints a credential**, at any verbosity or in any format.
- **The wrapper contributes environment variables and never removes them.** Ambient higher-precedence authentication is warned about, never stripped.
- **Login mode enforces a child version floor before spawn**; token mode and an unselected passthrough do not. `docs/decisions/0031-enforce-the-child-refresh-lock-version-floor.md`.
- **Selection appends one rung** below the configuration precedence ladder in `docs/reference/configuration.md`; it does not define a chain of its own.
- **Paths, modes, and writers** come from the artifact table in `docs/reference/xdg-storage.md`.

## Rejected Alternatives

- **A per-account credential seed copied into each session** — superseded. Separate copies bypass the child's refresh lock and recreate the rotate-and-revoke failure; see `docs/decisions/0025-share-one-native-login-per-account.md`.
- **Wrapper-side credential and project-trust sync-back** — removed with the seed model. `docs/decisions/0004-spawn-and-wait-child-supervision.md` is amended accordingly; spawn-and-wait now rests on child supervision and post-flight marker and log finalization.
- **A registry index file** — rejected; a second source of truth for which accounts exist drifts from the directory it claims to describe.
- **Wrapper-managed API-key or ambient-token injection as a fallback path** — rejected. Those mechanisms already outrank a subscription account inside the child; the wrapper reports them and stays out of the way.
- **Quota-aware auto-rotation and failover** — out of scope. It requires modelling quota state the wrapper cannot observe reliably, and it is orthogonal to isolation. See the switcher survey in `docs/reference/prior-art.md`.

## Risks & Edge Cases

- Login inside containers: the browser flow may be unavailable, which is what token mode is for; fail with a clear hint rather than a partial account.
- A pasted token minted earlier than ingest reports a wrong age unless the mint-time correction flag is used; the reported expiry is always labelled an estimate.
- The child's credential location and status-probe behaviour are externally owned and perishable; both are registered in `docs/reference/research-tracking.yaml` and consumed defensively.
- A below-floor child is a `doctor` warning and a `login`-mode launch failure — the same fact at two severities, because only one of them is a precondition.

## Completion

When all rounds are done, set each round `done` in this plan's `queue-rounds.yaml` and set this plan `done` in the top-level `.implementation-plans/queue-plans.yaml`. Nothing moves on disk.
