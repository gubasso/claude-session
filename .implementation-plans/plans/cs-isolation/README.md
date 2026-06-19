# claude-session — Session Isolation (pty-keyed, multiplexer-agnostic)

> Complexity: L | Rounds: 3 | Generated: 2026-06-19 | Repo: /workspaces/claude-session

## Problem Statement

Native `claude` shares session/config state across instances. `claude-session` must give every
interactive terminal its own fully isolated session by later pointing the child's `CLAUDE_CONFIG_DIR`
at a per-account, per-group isolated directory. The isolation method MUST be **general and
multiplexer-AGNOSTIC**: guarantee a distinct isolated session per interactive tab/split/pane, work
inside containers, and keep multiple `claude-session` instances inside the same container isolated —
**without sniffing for tmux/kitty/wezterm/screen/zellij**. The natural general key is the controlling
terminal (pty): every interactive pane owns a distinct pty regardless of multiplexer. This plan owns
the "compute the secure isolated session location + identity + context" domain. It does NOT spawn
`claude` or inject env — that is `cs-wrapper-runtime`'s job. Builds on `cs-foundation` (crate,
plumbing, figment config, error/ui).

## Strategy

Three rounds, bottom-up. R1 builds the secure filesystem primitives and XDG session-root resolution
(owner-checked, no-symlink, 0700). R2 builds the validated `AccountId`/`GroupId` newtypes and the
multiplexer-agnostic group derivation chain. R3 assembles the per-account/per-group session dir,
session metadata, lazy `AppContext` integration, and conservative stale-session cleanup.

## Rounds

1. `secure-session-dirs.md` — filesystem adapter + secure-dir service + XDG session-root resolution.
2. `group-identity.md` — `AccountId`/`GroupId` newtypes + multiplexer-agnostic derivation chain +
   neutral container discriminator.
3. `session-context-and-meta.md` — per-account/per-group session dir, `session-meta.json`, AppContext
   integration, stale-session cleanup.

## Execution Commands

```bash
# Execute the next todo round (executor reads queue-rounds.yaml, runs the first todo round, then stops):
/prex -ar @.implementation-plans/plans/cs-isolation/

# Or target a specific round file directly:
/prex -ar .implementation-plans/plans/cs-isolation/secure-session-dirs.md
```

## Execution Discipline

**Rounds must be executed one at a time.** Each round is a self-contained unit of work designed for a
single `/prex` session. Do not implement multiple rounds in one session.

When `/prex` is pointed at this directory or this `README.md`, it MUST:

1. Read this plan's `queue-rounds.yaml`.
2. Find the first round with status `todo`.
3. Set that round's `status` to `doing`, execute ONLY that round, then set it to `done` and stop.
4. End the session — a fresh `/prex` session is launched for any subsequent round.

## Decisions & Constraints

- `Executor: prex (EF 1.5)`.
- **Isolation is pty-keyed and multiplexer-AGNOSTIC.** Do NOT add
  `$TMUX_PANE`/`$KITTY_WINDOW_ID`/`$WEZTERM_PANE`/`$STY`/`$ZELLIJ_*` sniffing. The controlling terminal
  is the key — every interactive pane owns a distinct pty regardless of multiplexer.
- **GroupId derivation chain**: `--session`/`--group` flag → `CLAUDE_SESSION_GROUP` env → tty
  (`/dev/pts/3` → `pts-3`) → `ppid+starttime` (non-tty parents, `/proc/<ppid>/stat` field 22) →
  `pid-<PID>` with a visible warning. `GroupId` validation: ≤32 bytes, starts lowercase-ascii/digit,
  charset `[a-z0-9_-]`.
- **Cross-container collision edge**: only if a state dir is bind-mounted/shared across containers,
  namespace the key with ONE neutral host/container discriminator (`/etc/machine-id`, hostname, or a
  `/proc/self/cgroup`-derived id) — still general, still not multiplexer-aware.
- **`CLAUDE_CONFIG_DIR`** is the redirect var (analog of codex's `CODEX_HOME`); it is undocumented and
  leaky (bug #3833 can still create project-local `.claude/`). This plan resolves the dir; injection
  and leak handling land in `cs-wrapper-runtime`.
- **Session-root resolution**: prefer durable `$XDG_STATE_HOME` then `$XDG_RUNTIME_DIR`; document the
  choice in an ADR stub (codex prefers state→runtime; the shell tool runtime→state). Secure every dir:
  not a symlink, real dir, owned by current uid, chmod `0700`.
- Use a constant default account name (`default`) until `cs-accounts-auth` provides the registry.

## Rejected Alternatives

- **Multiplexer env-var sniffing** ($TMUX_PANE etc.) — rejected; not general, fails for plain ttys and
  unknown multiplexers.
- **Live symlinks into a shared `~/.claude`** (the shell tool's sync/link/home-link scheme) — rejected
  as intricate and bind-mount-fragile; the Rust rebuild uses self-contained per-account/per-group dirs.
- **`/tmp` fallback** — rejected (insecure); fall back to `pid-<id>` within the secure XDG root.
- **`bwrap`/`firejail` namespace sandbox** — deferred to a possible future "hardened synthetic-HOME
  mode"; out of this vision's scope.

## Risks & Edge Cases

- No controlling terminal (CI/headless): fall back to `pid-<PID>` with a visible warning; never crash.
- Bind-mounted shared state dir across containers: pty key alone can collide; apply the neutral
  discriminator only when that scenario is supported (document the boundary).
- Race between concurrent same-pane invocations: secure-dir creation must be idempotent and atomic.

## Completion

When all rounds are done, set each round `done` in this plan's `queue-rounds.yaml` and set this plan
`done` in the top-level `.implementation-plans/queue-plans.yaml`. Nothing moves on disk.
