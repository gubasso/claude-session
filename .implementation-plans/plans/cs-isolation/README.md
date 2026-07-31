# claude-session — Session Isolation (pty-keyed, multiplexer-agnostic)

> Complexity: L | Rounds: 3 | Generated: 2026-06-19 | Repo: repository root

## Problem Statement

Native `claude` shares session/config state across instances. `claude-session` must give every interactive terminal its own group directory, whose composed settings the child later receives through the native `--settings` flag (`docs/decisions/ADR-0028-pass-composed-settings-with-the-native-flag.md`). Account-wide native state stays shared; the split is explained in `docs/explanation/session-isolation.md`. The isolation method MUST be **general and multiplexer-AGNOSTIC**: guarantee a distinct isolated session per interactive tab/split/pane, work inside containers, and keep multiple `claude-session` instances inside the same container isolated — **without sniffing for tmux/kitty/wezterm/screen/zellij**. The natural general key is the controlling terminal (pty): every interactive pane owns a distinct pty regardless of multiplexer. This plan owns the "compute the secure isolated session location + identity + context" domain. It does NOT spawn `claude` or inject env — that is `cs-wrapper-runtime`'s job. Builds on `cs-foundation` (crate, plumbing, figment config, error/ui).

## Strategy

Three rounds, bottom-up. R1 builds the secure filesystem primitives and XDG session-root resolution (owner-checked, no-symlink, 0700). R2 builds the validated `AccountId`/`GroupId` newtypes and the multiplexer-agnostic group derivation chain. R3 assembles the per-account/per-group session dir, session metadata, lazy `AppContext` integration, and conservative stale-session cleanup.

## Rounds

1. `secure-session-dirs.md` — filesystem adapter + secure-dir service + XDG session-root resolution.
2. `group-identity.md` — `AccountId`/`GroupId` newtypes + multiplexer-agnostic derivation chain + neutral container discriminator.
3. `session-context-and-meta.md` — per-account/per-group session dir, `session-meta.json`, AppContext integration, stale-session cleanup.

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
- **Isolation is pty-keyed and multiplexer-AGNOSTIC.** Do NOT add `$TMUX_PANE`/`$KITTY_WINDOW_ID`/`$WEZTERM_PANE`/`$STY`/`$ZELLIJ_*` sniffing. The controlling terminal is the key — every interactive pane owns a distinct pty regardless of multiplexer.
- **GroupId derivation chain**: `--session` flag → `CLAUDE_SESSION_GROUP` env → tty (`/dev/pts/3` → `pts-3`) → `ppid+starttime` (non-tty parents, `/proc/<ppid>/stat` field 22) → `pid-<PID>` with a visible warning. `GroupId` validation: ≤32 bytes, starts lowercase-ascii/digit, charset `[a-z0-9_-]`.
- **Cross-container collision edge**: only if a state dir is bind-mounted/shared across containers, namespace the key with ONE neutral host/container discriminator (`/etc/machine-id`, hostname, or a `/proc/self/cgroup`-derived id) — still general, still not multiplexer-aware.
- **`CLAUDE_CONFIG_DIR`** selects the **account**'s native state, not the group's. It is externally owned and carries no documented stability guarantee, so it is tracked as a perishable fact in `docs/reference/research-tracking.yaml` and consumed defensively — the child can still create project-local state the variable does not cover. This plan resolves the account and group directories; injection lands in `cs-wrapper-runtime`.
- **Session roots live in the state base, with no runtime fallback.** Durable state never relocates into a directory cleared at logout or into a shared temporary directory — a fallback that can lose credentials is worse than a clear error. Decided in `docs/decisions/ADR-0006-place-files-by-xdg-ownership.md`; the artifact table is in `docs/reference/xdg-storage.md`. Secure every directory per component: not a symlink, real directory, owned by the current user, mode `0700` enforced on every run.
- Use a constant default account name (`default`) until `cs-accounts-auth` provides real account discovery.

## Rejected Alternatives

- **Multiplexer env-var sniffing** ($TMUX_PANE etc.) — rejected; not general, fails for plain ttys and unknown multiplexers.
- **Live symlinks into a shared child configuration directory** — rejected as intricate and bind-mount-fragile; this project uses self-contained per-account/per-group directories. Symlinks also let one session's write reach another, which is why `docs/reference/xdg-storage.md` requires a no-symlink check on every secured path.
- **A temporary-directory fallback** — rejected. A world-writable directory is the wrong home for a credential under any circumstances; the last rung of the identity chain is a process-id-keyed group **inside** the secure state root, not a different root.
- **`bwrap`/`firejail` namespace sandbox** — deferred to a possible future "hardened synthetic-HOME mode"; out of this vision's scope.

## Risks & Edge Cases

- No controlling terminal (CI/headless): fall back to `pid-<PID>` with a visible warning; never crash.
- Bind-mounted shared state dir across containers: pty key alone can collide; apply the neutral discriminator only when that scenario is supported (document the boundary).
- Race between concurrent same-pane invocations: secure-dir creation must be idempotent and atomic.

## Completion

When all rounds are done, set each round `done` in this plan's `queue-rounds.yaml` and set this plan `done` in the top-level `.implementation-plans/queue-plans.yaml`. Nothing moves on disk.
