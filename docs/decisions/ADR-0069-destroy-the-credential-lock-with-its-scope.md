# ADR-0069: Destroy the credential lock with the scope it guards

## Context and Problem Statement

`account remove` deletes an account tree that a concurrent `account login` may be writing into, and that tree contains `.credentials.lock`. [ADR-0060](./ADR-0060-lock-the-writes-that-are-not-derivable.md) makes "the lock file is never deleted" load-bearing, so removal as specified is both unsynchronized and in breach of the lock's own rule.

## Considered Options

- Delete everything but the lock, leaving an empty locked directory per removed account.
- Move the account lock outside the tree, to `accounts/.locks/<account>.lock`.
- Take the lock exclusively and delete the tree with the lock inside it.
- Add a lifecycle lock every account-backed launch holds for the child's whole life.

## Decision Outcome

Chosen option: **take the lock exclusively, then delete the tree with the lock inside it.** ADR-0060's rule protects a lock whose guarded files still exist; when the scope itself is destroyed the lock has nothing left to guard, and the remover holds it. A racer blocked on acquisition wakes holding an unlinked inode and fails its rename with `Io` — legible, and not a corrupted account.

The lock stays beside the files it guards, so the artifact table keeps one rule.

The lifecycle lock was rejected: a launch only reads, so locking it inverts ADR-0060's own criterion, and it would serialize nothing the wrapper needs serialized. Removal therefore detects no running child. One already running keeps working through its open descriptors and fails on its next start, which is what `rm` does and adds no failure mode stock `claude` lacks ([ADR-0058](./ADR-0058-behave-as-stock-claude-by-default.md)).

## Consequences

- Good: `remove` and `login` cannot interleave on one account.
- Good: removal leaves nothing behind, so no directory presents as a phantom account.
- Bad: `LockBusy` gains a second producer, so contention is no longer credential writes alone.
- Bad: a live session on a removed account is not stopped, and the report has to say so.

## Status

Accepted

Amends [ADR-0060](./ADR-0060-lock-the-writes-that-are-not-derivable.md) — the never-deleted rule holds while the scope exists; destroying the scope destroys its lock.
