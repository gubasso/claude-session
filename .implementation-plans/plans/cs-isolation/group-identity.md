# Isolation R2: Account/Group Identity (multiplexer-agnostic)

> Plan: cs-isolation | Round: 2 of 3 | Complexity: L | Executor: prex (EF 1.5) | Generated: 2026-06-19 | Repo: repository root

## Context

`claude-session` isolates every interactive terminal by giving each its own group directory under the selected account. The key must be general and multiplexer-AGNOSTIC: the controlling terminal (pty). Every interactive tab/split/pane owns a distinct pty regardless of tmux/kitty/wezterm/screen/zellij, so keying on the controlling terminal guarantees per-pane isolation without any multiplexer knowledge. This round builds the validated `AccountId` and `GroupId` newtypes and the derivation chain. Round 1 produced the secure filesystem primitives and session-root resolution.

## Previous Rounds

`cs-foundation`: crate tree, plumbing, `GlobalArgs` reserving `--session`/`--account`. This plan round 1: `adapters/fs.rs`, secure-dir service, `resolve_session_root`. Expect those to exist.

## Scope of This Round

- IN scope: `domain/ids.rs` (`AccountId` and `GroupId` newtypes; validation: lowercase-ASCII/digit start, `[a-z0-9_-]` body, length limits, returning `DomainError`); `services/session/group_id.rs` derivation chain with explicit priority (CLI `--session` → env `CLAUDE_SESSION_GROUP` → controlling terminal via `tty` → parent pid + `/proc/<ppid>/stat` starttime → process pid with a visible `Ui` warning); an optional neutral cross-container discriminator (machine-id / hostname / cgroup-derived) behind a config flag (off by default). Strictly NO multiplexer env-var sniffing.
- OUT of scope: building the group directory + metadata (round 3), account discovery and auth (`cs-accounts-auth`), spawning/env injection (`cs-wrapper-runtime`).

## Current State

### Key Files

- `src/domain.rs` (+ `src/domain/`) — add `ids.rs`.
- `src/services/session/` — add `group_id.rs`; consume `dir.rs`.
- `src/cli.rs` — `GlobalArgs` already reserves `--session`.

### Existing Patterns

The derivation chain and the reasoning for keeping it multiplexer-agnostic are in `docs/explanation/session-isolation.md`; the identifier rules are in `docs/reference/xdg-storage.md` — charset `[a-z0-9_-]`, a lowercase-ASCII or digit first character, at most 32 bytes.

Derive the terminal identity from the controlling terminal rather than by shelling out: query it directly through the process API (`rustix`) instead of running an external command, which is faster, avoids a `PATH` dependency, and cannot be confused by a shell function. Sanitize the device path into an identifier by stripping the `/dev/` prefix and replacing separators.

The non-tty rung reads the parent process id together with its **start time**, so a recycled process id cannot collide with a stale session. Read it through `rustix` rather than by hand-parsing where possible, and note that the field follows a parenthesized command name that may itself contain spaces or parentheses.

A derived value that fails validation is **rejected, not truncated** — truncation invites collision, which here means two terminals sharing a session directory.

The chain is `flag → env → controlling terminal → parent pid + start time → pid (with a visible warning)`. Keep it exactly this general; do NOT add multiplexer detection.

## Implementation Steps

### First Step: Mark this round as started

In this plan's `queue-rounds.yaml`, set this round's (`item: group-identity`) `status` to `doing`.

### Step 1: Newtypes

In `domain/ids.rs`, define validated `AccountId` and `GroupId` (constructor validation → `DomainError` on violation). Provide `as_str()` and `FromStr`.

### Step 2: Derivation chain

In `services/session/group_id.rs`, implement the priority chain (flag → env → tty → ppid+starttime → pid-with-warning), emitting a visible `Ui` warning on the pid fallback. Use injectable abstractions for `tty`/`/proc` where practical so it is unit-testable. NO multiplexer env-var sniffing.

### Step 3: Neutral container discriminator

Add an optional neutral discriminator (machine-id / hostname / cgroup-derived id) gated by a config flag (off by default), to namespace the group key only when state dirs may be shared across containers. Document the boundary in code comments (and reference the isolation ADR stub).

### Step 4: Tests

Unit-test `GroupId`/`AccountId` validation (accept `pts-3`, `ppid-1234-567890`, `pid-42`; reject empty/`FOO`/`foo/bar`/over-length) and the chain's priority ordering with injected inputs. Assert no multiplexer env var is read (grep/test guard).

### Final Step: Update the queue

1. In this plan's `queue-rounds.yaml`, set this round's (`item: group-identity`) `status` to `done`.

## Acceptance Criteria

- [ ] `GroupId`/`AccountId` reject invalid ids and accept the canonical forms.
- [ ] The chain returns a stable id for the same pty and distinct ids for distinct ptys; explicit CLI/env overrides win; no multiplexer env vars are read (test-verified).
- [ ] The non-interactive fallback warns via `Ui` and still yields an isolated id.
- [ ] This plan's `queue-rounds.yaml` shows round `group-identity` as `done`.

## Next Round

Round 3 (`session-context-and-meta`) uses these ids to build the secure `accounts/<account>/groups/
<group>/` session dir, writes `session-meta.json`, integrates lazy session resolution into `AppContext`, and adds conservative stale-session cleanup.
