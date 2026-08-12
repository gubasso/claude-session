# ADR-0061: Protect storage from accidental local drift

## Context and Problem Statement

The wrapper rejects linked, foreign-owned, or wrongly typed state paths and repairs modes, but no record says what the checks defend. [ADR-0056](./ADR-0056-classify-a-failed-spawn-by-its-cause.md), [ADR-0090](./ADR-0090-require-account-and-profile-before-child-launch.md), and storage review otherwise rely on an unnamed adversary model.

## Considered Options

- Leave it unwritten and argue each check on its own merits.
- A full threat model with enumerated adversary capabilities, as restic and BorgBackup publish.
- A short scope statement: the hazards in scope, and the one class explicitly out.

## Decision Outcome

Chosen option: a short scope statement. The deployment target is a single-user personal host, so these checks detect accident rather than defend against an adversary.

Assets are credentials, generated settings and provenance, and correct artifact identity.

In scope are permission drift, restored or migrated foreign ownership, sync tools replacing paths with links, and partial or reordered writes ([ADR-0060](./ADR-0060-lock-the-writes-that-are-not-derivable.md)).

Explicitly out of scope: a process running as this user. It can read the credential directly and needs no race, so hardening against it buys nothing. This is what settles the traversal mechanism — a non-following metadata pass then an ordinary open, never an `openat` descriptor walk, on exactly the reasoning ADR-0056 used against `fexecve`. Out of scope by inheritance after mandatory binding: anything stock `claude` is equally exposed to ([ADR-0090](./ADR-0090-require-account-and-profile-before-child-launch.md)).

Reconsideration trigger: a supported multi-user or shared-host deployment.

## Consequences

- Bad: a scope statement invites reading the checks as optional. They are not — accident is the common case.
- Bad: a shared-host deployment needs a superseding record before it is supported.

## Status

Accepted

Amended by [ADR-0065](./ADR-0065-retire-the-terminal-group.md) — the checks continue over the composed-settings store; session metadata and group identity leave the asset set.
