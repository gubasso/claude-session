# ADR-0108: Share the child's peer registry across sessions

## Context and Problem Statement

The child discovers peer sessions through pid-keyed registrations under its configuration directory's `sessions/` name, reaching each over a socket it names. Keying that directory by terminal ([ADR-0102](./ADR-0102-key-child-state-by-terminal.md)) splits the registry with everything else, so no two wrapper sessions see each other — though registrations interleave nothing, the property that keeps the shared `projects/` tree safe.

## Considered Options

- Leave the registry per terminal.
- Share one registry per account.
- Share one registry host-wide, scoped by boot and mount namespace, via a declared link.

## Decision Outcome

Chosen option: the host-wide scoped share — awareness is wanted across accounts, and wrapper storage defends against accident, not this user's own processes ([ADR-0061](./ADR-0061-protect-storage-from-accidental-local-drift.md)).

Each session directory's `sessions` name becomes a declared link ([ADR-0103](./ADR-0103-permit-a-declared-link.md)) to `peers/<boot>/<namespace>/` at the state root. The boot component derives from the kernel's boot identifier, the namespace component from the mount namespace, which carries the registry files and the sockets they name. Each answers half of the question whether a listed peer is reachable: distinct kernels report one fixed initial mount-namespace inode, so a virtual machine sharing the state tree stays apart only by boot, while containers of one kernel share its boot and stay apart only by namespace. Registrations die with their boot, so the scope churns with its contents.

A real directory already at the linked name is adopted: its entries move into the registry, replacement on collision being safe since liveness is decided at read time, and the emptied directory becomes the link. An underivable scope, or a registry the guard refuses, degrades to an unshared launch with a logged warning: awareness is additive, and a damaged shared convenience must not stop terminals from launching.

## Consequences

- Good: every session in one boot and mount view can list and message the others.
- Bad: accounts share one discovery surface, which strict isolation would not allow.
- Bad: the wrapper carries the child-owned name `sessions`, an ordinary word the fact scan cannot see, widening Q-012.

## Status

Implemented

Shaped by [031](../plan/slices/031-shared-peer-registry/README.md).
