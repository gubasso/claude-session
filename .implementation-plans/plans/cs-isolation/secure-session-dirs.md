# Isolation R1: Secure Filesystem Primitives & Session-Root Resolution

> Plan: cs-isolation | Round: 1 of 3 | Complexity: L | Executor: prex (EF 1.5) | Generated: 2026-06-19 |
> Repo: /workspaces/claude-session

## Context

`claude-session` stores isolated native `claude` state under XDG-controlled directories. Security
depends on refusing symlinks, enforcing current-uid ownership, and setting private (0700) permissions.
This round builds the secure filesystem adapter + service and the XDG session-root resolution that all
later isolation/auth/config rounds write into. The `cs-foundation` plan already provides the crate
tree, `AppError`/error layers, the figment `Config` (with resolved XDG paths), `Ui`, and `AppContext`.

## Previous Rounds

The `cs-foundation` plan (a depended-on sibling) produced: the canonical crate tree; `AppError` +
exit-code matrix and the per-layer error skeletons; tracing logging; single-writer `Ui`; immutable
`AppContext`; figment `Config` with XDG paths. Expect these to exist and compile.

## Scope of This Round

- IN scope: `adapters/fs.rs` (a filesystem adapter trait + default std/`rustix`/`tempfile` impl:
  `symlink_metadata`, secure `create_dir_all`, `chmod`, atomic write via tempfile+persist, rename);
  `services/session/dir.rs` secure-dir helpers (`ensure_owned_dir_0700`, `inspect_secure_dir`,
  step-by-step path creation) and `resolve_session_root(runtime_dir, state_dir)` (prefer durable
  `$XDG_STATE_HOME` then `$XDG_RUNTIME_DIR`, validate not-a-symlink/real-dir/owner-uid/0700) plus a
  non-mutating `inspect_session_root`; typed adapter/service errors (symlink, wrong-owner,
  non-directory, permission, io) mapping to `AppError` config/io/noperm sysexits.
- OUT of scope: deriving group ids (round 2), session dir/metadata (round 3), spawning or env
  injection (`cs-wrapper-runtime`), accounts (`cs-accounts-auth`).

## Current State

### Key Files

- `/workspaces/claude-session/src/adapters.rs` (+ `src/adapters/`) — add `fs.rs`.
- `/workspaces/claude-session/src/services.rs` (+ `src/services/`) — add `session/` submodule with
  `dir.rs`.
- `/workspaces/claude-session/src/error.rs` — extend with `#[from]` for the new adapter/service errors.

### Existing Patterns

Reference secure-dir logic (codex-session `services/session/dir.rs`, inspiration only): `secure_dir()`
calls `std::fs::symlink_metadata` and rejects symlinks; `create_dir_all`; rejects non-dir; checks
`metadata.uid() == current_uid()` (`rustix::process::getuid()`); forces
`Permissions::from_mode(0o700)`. Root preference: state then runtime. Add any blessed deps not yet present
(`rustix` with `process`,`fs`; `tempfile`) with **`cargo add`** (e.g.
`cargo add rustix --features process,fs`), never by hand-editing `[dependencies]`. Target disk layout (later rounds):
`$XDG_STATE_HOME/claude-session/accounts/<account>/groups/<group-id>/`.

## Implementation Steps

### First Step: Mark this round as started

In this plan's `queue-rounds.yaml`, set this round's (`item: secure-session-dirs`) `status` to
`doing`.

### Step 1: Filesystem adapter

In `adapters/fs.rs`, define a `Filesystem` trait (metadata, secure dir creation, chmod, atomic write,
rename) and a default impl over std + `rustix` + `tempfile`. Add an `<Sys>AdapterError` (thiserror,
`#[from] std::io::Error`).

### Step 2: Secure-dir service + session-root resolution

In `services/session/dir.rs`, implement `ensure_owned_dir_0700`, `inspect_secure_dir`, and
`resolve_session_root` (prefer `$XDG_STATE_HOME` then `$XDG_RUNTIME_DIR`; validate symlink/owner/mode)
plus non-mutating `inspect_session_root` for `doctor`. Add a `ServiceError`/`ConfigError` variant for
unresolvable roots.

### Step 3: Wire errors + tests

Wire the new errors into `AppError` with explicit `exit_code()` mappings (config/io/noperm). Unit-test
with `tempfile`: accept an owned 0700 dir; reject a symlinked dir and a wrong-mode dir; verify
state-preferred-over-runtime resolution.

### Final Step: Update the queue

1. In this plan's `queue-rounds.yaml`, set this round's (`item: secure-session-dirs`) `status` to
   `done`.

## Acceptance Criteria

- [ ] Secure dir creation rejects symlinks and wrong ownership (where detectable) and forces mode 0700.
- [ ] `resolve_session_root` prefers `$XDG_STATE_HOME`, falls back to `$XDG_RUNTIME_DIR`, and errors
      cleanly when neither is usable.
- [ ] New errors map to config/io/noperm sysexits; the exit-code matrix test still passes.
- [ ] Tests cover the happy path and symlink/wrong-owner rejection with `tempfile`.
- [ ] This plan's `queue-rounds.yaml` shows round `secure-session-dirs` as `done`.

## Next Round

Round 2 (`group-identity`) adds the validated `AccountId`/`GroupId` newtypes and the
multiplexer-agnostic derivation chain that names the per-group directory this round can securely
create.
