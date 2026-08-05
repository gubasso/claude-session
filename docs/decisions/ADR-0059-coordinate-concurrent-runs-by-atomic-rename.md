# ADR-0059: Coordinate concurrent runs by atomic rename, not by a lock

## Context and Problem Statement

The artifact table declared a `locks/<name>.lock` under the Runtime base, and [ADR-0033](./ADR-0033-append-fresh-exit-codes.md) gave `TempFail` (75) to a lock held by a concurrent run. No document names a lock, or a write that needs one — [ADR-0025](./ADR-0025-share-one-native-login-per-account.md) had already rejected a wrapper-wide lock, and the credential-refresh race it would have covered belongs to the child. A lock is also the one wrapper artifact whose kill-time remnant can refuse the next run, which is the failure mode [ADR-0058](./ADR-0058-behave-as-stock-claude-by-default.md) forbids.

## Considered Options

- Drop the lock; rely on the atomic rename and idempotent creation already specified.
- Keep the lock, held through an open descriptor with `flock`, so the kernel releases it on death.
- Keep the lock file and add a pid-and-boot-identifier liveness check the next run can break.

## Decision Outcome

Chosen option: **drop the lock** — every wrapper-owned write is either an atomic rename, where the loser of a race is overwritten by a complete file rather than corrupting one, or an idempotent directory creation. Neither has a critical section to protect, so mutual exclusion has no present subject ([ADR-0048](./ADR-0048-build-for-a-present-need.md)).

Conservative group pruning does not need it either: "possibly active" is decided by age, not by a held lock.

## Consequences

- Good: the stale-lock refusal stops existing rather than being detected and repaired, so the kill path has one less artifact class.
- Good: the Runtime base loses its only user and is removed, taking with it the "absent in containers, cron, and remote logins" degradation path and its report.
- Bad: `TempFail` (75) loses its only producer and leaves the matrix, so the availability argument in ADR-0033 no longer has a subject.
- Bad: a real need for mutual exclusion re-opens this, and arrives as a superseding record that names the write it protects.

## Status

Superseded

Superseded by [ADR-0060](./ADR-0060-lock-the-writes-that-are-not-derivable.md) — a minted credential and a two-file invariant are critical sections this record did not see, so the lock returns as an advisory `flock` the kernel releases on death. The atomic rename this record established is unchanged and still carries every write.
