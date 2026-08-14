# ADR-0101: Retire the credential a mode switch supersedes

## Context and Problem Statement

An account records one authentication mode, and a login that changed it recorded the new mode and stopped. The artifact the old mode used stayed on disk with nothing able to reach it: every read is gated on the recorded mode, no rotation refreshes it, and only removing the whole account cleared it. A stored token outlives its mode by up to a year that way.

## Considered Options

- Unlink the superseded artifact inside the commit that records the new mode.
- Leave it, and report it through `doctor` and `account status`.
- Refuse a switch until the account is removed.

## Decision Outcome

Chosen option: unlink it inside the commit — a credential nothing can reach is a hazard rather than a spare, and that commit is the only moment knowing a switch happened, and already holds the lock.

The unlink is decided by the mode just recorded rather than by the mode replaced, so it reads no metadata this run is overwriting, and its idempotence repairs an account orphaned before this record. It follows the metadata rename and never precedes it: the rename is the commit, so an interruption before it must leave the previous credential working rather than the account holding neither. A failure fails the login without undoing it, and names the path that still holds a credential.

## Consequences

- Good: a switch leaves exactly one credential, and an older orphan is cleared by that account's next login rather than a check the reader runs.
- Good: the report names an irreversible act, and says nothing when none happened.
- Bad: the wrapper unlinks a child-owned credential, which [accounts](../reference/accounts.md) had described as hands-off. That page now names removal and this as the two exceptions, rather than letting its silence read as a promise.
- Bad: neither credential is revoked at the provider. Both stay valid until they expire, because the wrapper speaks no endpoint ([ADR-0026](./ADR-0026-store-and-inject-a-long-lived-subscription-token.md)).

## Status

Implemented

Delivered by [slice 027](../plan/slices/027-superseded-credential-retirement/README.md), in `retire_superseded` and the two commits calling it.
