# ADR-0067: Commit a token rotation with the metadata rename

## Context and Problem Statement

A token-mode account is two files, `oauth-token` and `auth-mode.json`, and a rotation rewrites both. [ADR-0026](./ADR-0026-store-and-inject-a-long-lived-subscription-token.md) required the replacement to be staged and verified, which the accounts reference read as a durable staging file and a discard step; [the atomic sequence](../reference/xdg-storage.md#the-sequence) has neither, and names no order for the two renames. A crash between them therefore had no defined outcome, and `account status`, the `credentials-usable` check, and every token-mode launch read the pair.

## Considered Options

- A durable staging file, verified in place, then renamed over the token.
- Rename `auth-mode.json` first and `oauth-token` last.
- Rename `oauth-token` first and `auth-mode.json` last, verifying before either.

## Decision Outcome

Chosen option: **verify, then rename `oauth-token`, then `auth-mode.json`, whose rename commits the rotation.** The candidate is probed by the child's own documented status command with the token in the child environment, so verification needs no file on disk and the credential lock is never held across a child spawn.

The order follows from which half-written state is survivable. Metadata first leaves a fingerprint describing a token that is not there, over a token the exchange may already have revoked. Token first inverts that: the account holds a credential just proven to work, described by stale metadata, which the recorded fingerprint makes detectable and the next `account login` repairs.

No durable staging artifact exists. The atomic-write temporary is consumed by its own rename, so there is nothing to discard, no row to add to the artifact table, no sweep rule, and nothing for `account remove` to leave behind.

## Consequences

- Good: a failure at any point leaves the account usable, which is what rotation is for.
- Good: the pair is written entirely by the existing sequence and lock scope; no new mechanism.
- Bad: a crash between the renames leaves age and estimated expiry stale until repaired.

## Status

Accepted

Amends [ADR-0026](./ADR-0026-store-and-inject-a-long-lived-subscription-token.md) — the replacement is still verified before the old token is replaced, but staging is the atomic-write temporary rather than an artifact of its own. The order and its lock scope live in [XDG storage](../reference/xdg-storage.md#lock-scopes).
