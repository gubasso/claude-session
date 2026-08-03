# claude-session — Session Isolation (account and profile scopes)

> Complexity: L | Rounds: 3 | Generated: 2026-06-19 | Repo: repository root

## Problem Statement

Native `claude` shares session and config state across instances. `claude-session` separates two scopes: an account owns the shared child `config/`, and a **profile** owns the composed settings the child later receives through the native `--settings` flag (`docs/decisions/ADR-0028-pass-composed-settings-with-the-native-flag.md`). The split is explained in `docs/explanation/session-isolation.md`. Composed settings are keyed by the inputs they were composed from — the profile, its ordered pieces, and their contents — and never by a terminal, a working directory, or a project (`docs/decisions/ADR-0064-key-composed-settings-by-profile-and-input-digest.md`). This plan owns the "compute the secure storage location, the identifiers, and the context" domain. It does NOT spawn `claude` or inject env — that is `cs-wrapper-runtime`'s job. Builds on `cs-foundation` (crate, plumbing, figment config, error/ui).

## Strategy

Three rounds, bottom-up. R1 builds the secure filesystem primitives and XDG session-root resolution (owner-checked, no-symlink, 0700). R2 builds the validated `AccountId`/`ProfileId` newtypes, the composed-settings entry key, and its write-once publication rule. R3 assembles the account directory and lazy `AppContext` resolution of the account and the profile.

## Rounds

1. `secure-session-dirs.md` — filesystem adapter + secure-dir service + XDG session-root resolution.
2. `composed-settings-store.md` — `AccountId`/`ProfileId` newtypes + input-digest entry key + write-once publication.
3. `session-context.md` — account directory, lazy account and profile resolution in `AppContext`, `doctor` paths.

## Execution Commands

Any executor following [the contract](../../README.md#the-executor-contract) can run these rounds. `/prex` is the one used to generate them, shown here as a worked example:

```bash
# Execute the next todo round (executor reads queue-rounds.yaml, runs the first todo round, then stops):
/prex -ar @.implementation-plans/plans/cs-isolation/

# Or target a specific round file directly:
/prex -ar .implementation-plans/plans/cs-isolation/secure-session-dirs.md
```

## Execution Discipline

Execution follows the executor contract in [`../../README.md`](../../README.md#the-executor-contract), which owns the rule: one round per session, first `todo` round only, status transitions in `queue-rounds.yaml`, stop.

This plan adds no exceptions to it.

## Decisions & Constraints

- **Executor provenance:** `prex (EF 1.5)` — the profile these rounds were generated with. Provenance only; see [the contract](../../README.md#the-executor-contract).
- **Isolation is keyed by composition inputs.** No terminal, multiplexer, emulator, container, machine-identity, working-directory, or project input enters any path. Two profiles launched from one pane name different entries; identical inputs from different panes name one.
- **Entry key**: `profile-<name>-<12 hex>.json` and its `.compose.json` sidecar under `composed/`, the digest taken over the profile name, the profile file, and every piece by content. `AccountId`/`ProfileId` validation: ≤ 32 bytes, starts lowercase-ascii/digit, charset `[a-z0-9_-]`.
- **Cross-container collision edge**: dissolved. A shared or bind-mounted state directory is safe, because an entry's name is a function of its content and identical names carry identical bytes.
- **`CLAUDE_CONFIG_DIR`** selects the **account**'s native state, not the profile's. It is externally owned and carries no documented stability guarantee, so it is tracked as a perishable fact in `docs/reference/research-tracking.yaml` and consumed defensively — the child can still create project-local state the variable does not cover. This plan resolves the account directory and the composed-settings entry; injection lands in `cs-wrapper-runtime`.
- **Session roots live in the state base, with no runtime fallback.** Durable state never relocates into a directory cleared at logout or into a shared temporary directory — a fallback that can lose credentials is worse than a clear error. Decided in `docs/decisions/ADR-0006-place-files-by-xdg-ownership.md`; the artifact table is in `docs/reference/xdg-storage.md`. Secure every directory per component: not a symlink, real directory, owned by the current user, mode `0700` enforced on every run.
- Use a constant default account name (`default`) until `cs-accounts-auth` provides real account discovery.

## Rejected Alternatives

- **Any terminal-derived key** (a pty name, a multiplexer or emulator variable, a login session id) — rejected; composition has no terminal input, so keying by one lets two profiles in one pane collide (`docs/decisions/ADR-0065-retire-the-terminal-group.md`).
- **Live symlinks into a shared child configuration directory** — rejected as intricate and bind-mount-fragile; this project uses a self-contained per-account directory and a separate composed-settings store. Symlinks also let one session's write reach another, which is why `docs/reference/xdg-storage.md` requires a no-symlink check on every secured path.
- **A temporary-directory fallback**, and a fresh temporary directory per run — rejected. A world-writable directory is the wrong home for a credential under any circumstances, durable state never falls back to one (`docs/reference/xdg-storage.md`), and a per-run directory recomposes on every launch and leaks a directory no later run can attribute.
- **`bwrap`/`firejail` namespace sandbox** — deferred to a possible future "hardened synthetic-HOME mode"; out of this vision's scope.

## Risks & Edge Cases

- No controlling terminal (CI/headless): nothing derives one, so this is not a case — the entry name is a function of the composition inputs alone.
- Race between concurrent invocations: secure-dir creation must be idempotent and atomic, and entry publication follows the rule in `docs/reference/xdg-storage.md`.
- A truncated digest is twelve hex characters, so the sidecar's recorded full digest — not the filename — is what proves an existing entry belongs to this run's inputs.

## Completion

When all rounds are done, set each round `done` in this plan's `queue-rounds.yaml` and set this plan `done` in the top-level `.implementation-plans/queue-plans.yaml`. Nothing moves on disk.
