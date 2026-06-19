# Isolation R2: Account/Group Identity (multiplexer-agnostic)

> Plan: cs-isolation | Round: 2 of 3 | Complexity: L | Executor: prex (EF 1.5) | Generated: 2026-06-19 |
> Repo: /workspaces/claude-session

## Context

`claude-session` isolates every interactive terminal by giving each its own `CLAUDE_CONFIG_DIR` (set
later by `cs-wrapper-runtime`). The key must be general and multiplexer-AGNOSTIC: the controlling
terminal (pty). Every interactive tab/split/pane owns a distinct pty regardless of
tmux/kitty/wezterm/screen/zellij, so keying on the controlling terminal guarantees per-pane isolation
without any multiplexer knowledge. This round builds the validated `AccountId` and `GroupId` newtypes
and the derivation chain. Round 1 produced the secure filesystem primitives and session-root
resolution.

## Previous Rounds

`cs-foundation`: crate tree, plumbing, `GlobalArgs` reserving `--session`/`--group`/`--account`. This
plan round 1: `adapters/fs.rs`, secure-dir service, `resolve_session_root`. Expect those to exist.

## Scope of This Round

- IN scope: `domain/ids.rs` (`AccountId` and `GroupId` newtypes; validation: lowercase-ASCII/digit
  start, `[a-z0-9_-]` body, length limits, returning `DomainError`); `services/session/group_id.rs`
  derivation chain with explicit priority (CLI `--session`/`--group` → env `CLAUDE_SESSION_GROUP` →
  controlling terminal via `tty` → parent pid + `/proc/<ppid>/stat` starttime → process pid with a
  visible `Ui` warning); an optional neutral cross-container discriminator (machine-id / hostname /
  cgroup-derived) behind a config flag (off by default). Strictly NO multiplexer env-var sniffing.
- OUT of scope: building the session dir + metadata (round 3), account registry/auth
  (`cs-accounts-auth`), spawning/env injection (`cs-wrapper-runtime`).

## Current State

### Key Files

- `/workspaces/claude-session/src/domain.rs` (+ `src/domain/`) — add `ids.rs`.
- `/workspaces/claude-session/src/services/session/` — add `group_id.rs`; consume `dir.rs`.
- `/workspaces/claude-session/src/cli/mod.rs` — `GlobalArgs` already reserves `--session`/`--group`.

### Existing Patterns

Reference tty derivation (codex-session `group_id.rs`, inspiration only):

```rust
fn current_from_tty() -> Option<GroupId> {
    let output = std::process::Command::new("tty").output().ok()?;
    if !output.status.success() { return None; }
    let tty = String::from_utf8(output.stdout).ok()?;
    let stripped = tty.trim().strip_prefix("/dev/")?;   // "/dev/pts/3" -> "pts/3"
    let id = stripped.replace('/', "-");                 // -> "pts-3"
    if id.len() > 64 { return None; }
    Some(GroupId::from_unchecked(id))
}
```

ppid fallback reads `/proc/<ppid>/stat` field 22 (starttime, after the parenthesized `comm`) via
`rustix`. `GroupId` validation: ≤32 bytes, leading lowercase-ascii/digit, charset `[a-z0-9_-]`. The
reference chain is `flag → env → tty → ppid → pid(warn)` — keep it exactly this general; do NOT add
multiplexer detection.

## Implementation Steps

### First Step: Mark this round as started

In this plan's `queue-rounds.yaml`, set this round's (`item: group-identity`) `status` to `doing`.

### Step 1: Newtypes

In `domain/ids.rs`, define validated `AccountId` and `GroupId` (constructor validation → `DomainError`
on violation). Provide `as_str()` and `FromStr`.

### Step 2: Derivation chain

In `services/session/group_id.rs`, implement the priority chain (flag → env → tty → ppid+starttime →
pid-with-warning), emitting a visible `Ui` warning on the pid fallback. Use injectable abstractions for
`tty`/`/proc` where practical so it is unit-testable. NO multiplexer env-var sniffing.

### Step 3: Neutral container discriminator

Add an optional neutral discriminator (machine-id / hostname / cgroup-derived id) gated by a config
flag (off by default), to namespace the group key only when state dirs may be shared across containers.
Document the boundary in code comments (and reference the isolation ADR stub).

### Step 4: Tests

Unit-test `GroupId`/`AccountId` validation (accept `pts-3`, `ppid-1234-567890`, `pid-42`; reject
empty/`FOO`/`foo/bar`/over-length) and the chain's priority ordering with injected inputs. Assert no
multiplexer env var is read (grep/test guard).

### Final Step: Update the queue

1. In this plan's `queue-rounds.yaml`, set this round's (`item: group-identity`) `status` to `done`.

## Acceptance Criteria

- [ ] `GroupId`/`AccountId` reject invalid ids and accept the canonical forms.
- [ ] The chain returns a stable id for the same pty and distinct ids for distinct ptys; explicit
      CLI/env overrides win; no multiplexer env vars are read (test-verified).
- [ ] The non-interactive fallback warns via `Ui` and still yields an isolated id.
- [ ] This plan's `queue-rounds.yaml` shows round `group-identity` as `done`.

## Next Round

Round 3 (`session-context-and-meta`) uses these ids to build the secure `accounts/<account>/groups/
<group>/` session dir, writes `session-meta.json`, integrates lazy session resolution into
`AppContext`, and adds conservative stale-session cleanup.
