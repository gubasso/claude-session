# ADR-0060: Lock the writes that are not derivable

## Context and Problem Statement

[ADR-0059](./ADR-0059-coordinate-concurrent-runs-by-atomic-rename.md) dropped locks because derived writes have no critical section: concurrent runs produce identical bytes. Minted `oauth-token` and `auth-mode.json` do not; reversed renames can persist a revoked credential. Atomic rename prevents partial reads, not reordering.

## Considered Options

- Keep the rename alone and accept the reordering.
- Git's lockfile: `<file>.lock` created with `O_CREAT | O_EXCL`, serving as both lock and temporary, released by the rename that commits it.
- An advisory `flock` on a permanent sentinel, with the atomic rename performed inside it.

## Decision Outcome

Chosen option: an advisory lock around the atomic write. The kernel releases `flock` on holder death, including `SIGKILL`, so the post-binding native-behaviour rule [ADR-0090](./ADR-0090-require-account-and-profile-before-child-launch.md) preserves forbids a stale-lock refusal. An `O_EXCL` lockfile needs cleanup a killed process cannot run.

`std::fs::File::lock` is stable since Rust 1.89 against an MSRV of 1.97, so this costs no dependency.

The permanent lock file outlives its holders. Because `flock` is per open file description, an in-process mutex sits above it. See [lock scopes](../reference/xdg-storage.md#lock-scopes).

Scope is the writes that need it — the account credential pair and the settings-and-provenance pair. A write derived from its inputs stays lock-free.

## Consequences

- Bad: a permanent zero-byte lock file per scope, and a deadline to choose.
- Bad: `flock` is unreliable over NFS, where a state directory is already ill-advised.

## Status

Implemented

Enacted by [the lock scopes](../reference/xdg-storage.md#lock-scopes).

Supersedes [ADR-0059](./ADR-0059-coordinate-concurrent-runs-by-atomic-rename.md); atomic rename remains. Amended by [ADR-0064](./ADR-0064-key-composed-settings-by-profile-and-input-digest.md) to withdraw the settings scope and [ADR-0069](./ADR-0069-destroy-the-credential-lock-with-its-scope.md) to destroy an account lock with its scope.
