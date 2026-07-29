# claude-session — Accounts & Subscription Auth

> Complexity: L | Rounds: 4 | Generated: 2026-06-19 | Repo: repository root

## Problem Statement

`claude-session` must support multiple named accounts with **subscription auth as the primary and always-available path**, while keeping native `claude` config blind to the user. It mirrors the reference's three-layer auth model: a managed `claude login` per account authenticates into an isolated dir → the resulting credentials become a per-account **seed** → each session gets its own **copy** of that seed. API-key / `ANTHROPIC_AUTH_TOKEN` injection is a secondary fallback for headless/CI. This plan adds the account registry, the managed-login flow, the resolver + auth gate (seed→session copy), the trust sync-back of `.claude.json` project state, and the `account` CLI verbs. Depends on `cs-wrapper-runtime` (managed login and the gate spawn `claude` via the runtime) and, transitively, `cs-isolation`/`cs-foundation`.

## Strategy

Four rounds. R1 builds the filesystem-backed account registry (`accounts/<name>/`, seed paths, `last-account`). R2 implements managed `claude login` per account into an isolated dir + hardened credential copy to the seed. R3 builds the resolver (flag > env > last/default) + the auth gate (seed→session copy) + API-key fallback + trust sync-back. R4 exposes the `account` CLI verbs with `--format json` and redaction, and wires `doctor`.

## Rounds

1. `account-registry.md` — filesystem-backed registry, account dirs, seed paths, last-account.
2. `managed-login.md` — managed `claude login` per account (subscription OAuth) + hardened seed write.
3. `auth-gate-and-resolver.md` — resolver, auth gate, seed→session copy, API-key fallback, trust sync-back.
4. `account-commands.md` — `account add|list|current|remove|refresh` verbs, `--format json`, redaction, doctor.

## Execution Commands

Any executor following [the contract](../../README.md#the-executor-contract) can run these rounds. `/prex` is the one used to generate them, shown here as a worked example:

```bash
# Execute the next todo round (executor reads queue-rounds.yaml, runs the first todo round, then stops):
/prex -ar @.implementation-plans/plans/cs-accounts-auth/

# Or target a specific round file directly:
/prex -ar .implementation-plans/plans/cs-accounts-auth/account-registry.md
```

## Execution Discipline

Execution follows the executor contract in [`../../README.md`](../../README.md#the-executor-contract), which owns the rule: one round per session, first `todo` round only, status transitions in `queue-rounds.yaml`, stop.

This plan adds no exceptions to it.

## Decisions & Constraints

- **Executor provenance:** `prex (EF 1.5)` — the profile these rounds were generated with. Provenance only; see [the contract](../../README.md#the-executor-contract).
- **Subscription auth is primary and MUST always be available** via managed `claude login`. API-key / `ANTHROPIC_AUTH_TOKEN` injection is a secondary fallback only.
- **Three-layer auth model**: managed login into an isolated dir → per-account seed (`accounts/<name>/`) → per-session copy (into the session dir from `cs-isolation`). Mirrors codex's native→seed→session.
- **Account selection priority**: CLI `--account` flag > env `CLAUDE_SESSION_ACCOUNT` > last-used / `default`. `AccountId` reuses the validated newtype from `cs-isolation`.
- **Native config blind to user**: users never edit native `claude` credential files directly; claude-session manages seeds and session copies. Credentials are never written to user-editable config.
- **Hardened credential I/O**: symlink/hardlink/ownership checks, `secure_file_read`, atomic writes, 0700 dirs / 0600 files; never write a token into `~/.config/claude-session` or a cleanable session cache path.
- **Native credential surface**: `claude` writes `.credentials.json` under its config dir (or macOS Keychain); a fresh isolated `CLAUDE_CONFIG_DIR` has none until login or token injection.

## Rejected Alternatives

- **API-key-only auth** — rejected; subscription must be primary and always available.
- **Mixing `apiKeyHelper` with subscription tokens** — rejected (the shell tool flags this as fragile and surprising); pick one path per invocation.
- **Storing tokens in user config** — rejected; credentials live only in secured seed/session dirs.
- **Quota-aware auto-rotation and failover** — out of scope for v1. It requires modelling quota state the wrapper cannot observe reliably, and it is orthogonal to isolation. The resolver supports explicit and last/default selection, leaving room for a future `auto`. See the switcher survey in `docs/reference/prior-art.md`.

## Risks & Edge Cases

- Managed login inside containers: the OAuth/browser flow may not be available; document the API-key / token fallback path and fail with a clear hint.
- Seed staleness/expiry: `account refresh` re-runs login; the gate surfaces a clear error when a seed is missing.
- Concurrent session copies of the same seed: copy must be atomic and idempotent.
- Trust sync-back must not clobber a newer `.claude.json` state; merge conservatively under a lock.

## Completion

When all rounds are done, set each round `done` in this plan's `queue-rounds.yaml` and set this plan `done` in the top-level `.implementation-plans/queue-plans.yaml`. Nothing moves on disk.
