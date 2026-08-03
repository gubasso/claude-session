# ADR-0061: Protect storage from accidental local drift

## Context and Problem Statement

The wrapper refuses to run on a state path that is a symbolic link, is owned by another user, or has the wrong file type, and repairs a wrong mode in place. Nothing records what that policy defends against. Three decisions already argue from the unwritten premise — [ADR-0056](./ADR-0056-classify-a-failed-spawn-by-its-cause.md) rejects `fexecve` because the wrapper is unprivileged, [ADR-0058](./ADR-0058-behave-as-stock-claude-by-default.md) bounds the failure modes the wrapper may add, and the storage checks assert a policy with no named adversary. So the argument is re-run at every review, and the next reader can reasonably conclude that a capability-based traversal library is missing.

## Considered Options

- Leave it unwritten and argue each check on its own merits.
- A full threat model with enumerated adversary capabilities, as restic and BorgBackup publish.
- A short scope statement: the hazards in scope, and the one class explicitly out.

## Decision Outcome

Chosen option: **a short scope statement**. The deployment target is a single-user personal host, so these checks detect accident rather than defend against an adversary.

Assets: account credentials, generated settings and provenance, session metadata, and correct artifact identity.

In scope: permission drift from a recursive `chmod`; a restored backup or migrated tree under the wrong owner; a file-sync or snapshot tool that replaces a path with a link or mutates the tree mid-run; partial or reordered wrapper writes, already answered by [ADR-0060](./ADR-0060-lock-the-writes-that-are-not-derivable.md).

Explicitly out of scope: **a process running as this user**. It can read the credential directly and needs no race, so hardening against it buys nothing. This is what settles the traversal mechanism — a non-following metadata pass then an ordinary open, never an `openat` descriptor walk, on exactly the reasoning ADR-0056 used against `fexecve`. Out of scope by inheritance: anything stock `claude` is equally exposed to (ADR-0058).

Reconsideration trigger: a supported multi-user or shared-host deployment.

## Consequences

- Good: every storage check has a stated reason, and a proposal to harden further has a criterion to fail.
- Good: a capability-traversal dependency is pre-rejected, so the dependency set stays as it is.
- Bad: a scope statement invites reading the checks as optional. They are not — accident is the common case.
- Bad: a shared-host deployment needs a superseding record before it is supported.

## Status

Accepted
