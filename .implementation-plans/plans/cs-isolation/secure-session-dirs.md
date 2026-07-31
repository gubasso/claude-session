# Isolation R1: Secure Filesystem Primitives & Session-Root Resolution

> Plan: cs-isolation | Round: 1 of 3 | Complexity: L | Executor: prex (EF 1.5) | Generated: 2026-06-19 | Repo: repository root

## Context

`claude-session` stores isolated native `claude` state under XDG-controlled directories. Security depends on refusing symlinks, enforcing current-uid ownership, and setting private (0700) permissions. This round builds the secure filesystem adapter + service and the XDG session-root resolution that all later isolation/auth/config rounds write into. The `cs-foundation` plan already provides the crate tree, `AppError`/error layers, the figment `Config` (with resolved XDG paths), `Ui`, and `AppContext`.

## Previous Rounds

The `cs-foundation` plan (a depended-on sibling) produced: the canonical crate tree; `AppError` + exit-code matrix and the per-layer error skeletons; tracing logging; single-writer `Ui`; immutable `AppContext`; figment `Config` with XDG paths. Expect these to exist and compile.

## Scope of This Round

- IN scope: `adapters/fs.rs` (a filesystem adapter trait + default std/`rustix`/`tempfile` impl: no-follow metadata, secure directory creation, mode enforcement, atomic write via temporary-file-then-rename **in the same directory**, rename); `services/session/dir.rs` secure-dir helpers (`ensure_owned_dir_0700`, `inspect_secure_dir`, step-by-step per-component path creation) and `resolve_session_root(state_dir)` (the state base; validate not-a-symlink, real directory, owner, mode `0700`) plus a non-mutating `inspect_session_root`; typed adapter/service errors (symlink, wrong-owner, non-directory, permission, io) mapping to the codes in `docs/reference/exit-codes.md`.
- OUT of scope: deriving group ids (round 2), session dir/metadata (round 3), spawning or env injection (`cs-wrapper-runtime`), accounts (`cs-accounts-auth`).

## Current State

### Key Files

- `src/adapters.rs` (+ `src/adapters/`) — add `fs.rs`.
- `src/services.rs` (+ `src/services/`) — add `session/` submodule with `dir.rs`.
- `src/error.rs` — extend with `#[from]` for the new adapter/service errors.

### Existing Patterns

The security posture is specified in `docs/reference/xdg-storage.md` and must be implemented as written: a metadata call that does **not** follow symbolic links, applied to **each path component in turn** rather than only to the leaf; ownership checked against the current user; non-directories rejected; mode enforced at `0700` on **every** invocation rather than assumed from creation. Creation is idempotent, since two invocations from the same pane can race.

**Session roots resolve to the state base, and there is no fallback into runtime.** The runtime base has no portable default and is genuinely absent in containers and under `cron`; a fallback that relocates durable state there can lose credentials and would present as a mysterious logout. When runtime is unavailable the wrapper degrades explicitly and durable state stays put. See `docs/decisions/ADR-0006-place-files-by-xdg-ownership.md`. Runtime is used only for locks, and a missing runtime base disables locking rather than moving it.

Add any crates not yet present (`rustix` with `process` and `fs`; `tempfile`) with **`cargo add`** — never by hand-editing `[dependencies]`. Target disk layout for later rounds is the artifact table in `docs/reference/xdg-storage.md`.

## Implementation Steps

### First Step: Mark this round as started

In this plan's `queue-rounds.yaml`, set this round's (`item: secure-session-dirs`) `status` to `doing`.

### Step 1: Filesystem adapter

In `adapters/fs.rs`, define a `Filesystem` trait (metadata, secure dir creation, chmod, atomic write, rename) and a default impl over std + `rustix` + `tempfile`. Add an `<Sys>AdapterError` (thiserror, `#[from] std::io::Error`).

### Step 2: Secure-dir service + session-root resolution

In `services/session/dir.rs`, implement `ensure_owned_dir_0700`, `inspect_secure_dir`, and `resolve_session_root` (the state base; validate symlink, owner, and mode per component) plus a non-mutating `inspect_session_root` for `doctor`. Add an error variant for an unresolvable root. Do **not** implement a runtime fallback for durable state; an unusable state base is a clear error, not a reason to relocate credentials.

### Step 3: Wire errors + tests

Wire the new errors into `AppError` with explicit `exit_code()` mappings (config/io/noperm). Unit-test with `tempfile`: accept an owned 0700 dir; reject a symlinked dir and a wrong-mode dir; verify state-preferred-over-runtime resolution.

### Final Step: Update the queue

1. In this plan's `queue-rounds.yaml`, set this round's (`item: secure-session-dirs`) `status` to `done`.

## Acceptance Criteria

- [ ] Secure dir creation rejects symlinks and wrong ownership (where detectable) and forces mode 0700.
- [ ] `resolve_session_root` resolves the state base and errors cleanly when it is unusable; it never relocates durable state into the runtime base or a temporary directory.
- [ ] New errors map to config/io/noperm sysexits; the exit-code matrix test still passes.
- [ ] Tests cover the happy path and symlink/wrong-owner rejection with `tempfile`.
- [ ] This plan's `queue-rounds.yaml` shows round `secure-session-dirs` as `done`.

## Next Round

Round 2 (`group-identity`) adds the validated `AccountId`/`GroupId` newtypes and the multiplexer-agnostic derivation chain that names the per-group directory this round can securely create.
