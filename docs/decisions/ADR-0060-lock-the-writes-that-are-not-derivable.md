# ADR-0060: Lock the writes that are not derivable

## Context and Problem Statement

[ADR-0059](./ADR-0059-coordinate-concurrent-runs-by-atomic-rename.md) dropped the lock on the grounds that no wrapper write has a critical section. That holds for a write recomputed from on-disk inputs: two runs produce identical bytes, so last-writer-wins is invisible. It fails twice. `oauth-token` and `auth-mode.json` are minted by an external exchange rather than derived, so renames landing out of issue order persist an already-revoked credential. Generated settings and their provenance are two files carrying one invariant. An atomic rename buys integrity — no reader sees a partial file — never preservation.

## Considered Options

- Keep the rename alone and accept the reordering.
- Git's lockfile: `<file>.lock` created with `O_CREAT | O_EXCL`, serving as both lock and temporary, released by the rename that commits it.
- An advisory `flock` on a permanent sentinel, with the atomic rename performed inside it.

## Decision Outcome

Chosen option: **an advisory lock around the atomic write**. The kernel releases a `flock` when the holder dies for any reason, `SIGKILL` included, so the stale-lock refusal [ADR-0058](./ADR-0058-behave-as-stock-claude-by-default.md) forbids cannot arise — the objection that removed the lock in ADR-0059, answered rather than avoided. Git's `O_EXCL` lockfile has no such release and needs the exit handlers a killed process never runs.

`std::fs::File::lock` is stable since Rust 1.89 against an MSRV of 1.97, so this costs no dependency.

Two properties are load-bearing. The lock file is **never deleted**, because unlinking it lets one holder destroy the file another is locking. And a `flock` is held per open file description rather than per process, so an in-process mutex sits above it or two threads in one wrapper defeat it.

Scope is the writes that need it — the account credential pair and the settings-and-provenance pair. A write derived from its inputs stays lock-free.

Concurrent `account login` on one account still discards one token. The lock makes the survivor the last issued, not both.

## Consequences

- Good: `TempFail` (75) regains the producer [ADR-0033](./ADR-0033-append-fresh-exit-codes.md) anticipated — an acquisition past its deadline.
- Good: settings-before-provenance ordering becomes one critical section rather than a rule to remember.
- Bad: a permanent zero-byte lock file per scope, and a deadline to choose.
- Bad: `flock` is unreliable over NFS, where a state directory is already ill-advised.

## Status

Accepted

Supersedes [ADR-0059](./ADR-0059-coordinate-concurrent-runs-by-atomic-rename.md) — the atomic rename and its temporary naming carry forward unchanged; only "no wrapper write has a critical section" is withdrawn. The runtime base stays unused, since a lock lives beside the file it guards.
