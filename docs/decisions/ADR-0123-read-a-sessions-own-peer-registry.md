# ADR-0123: Read a session's own peer registry

## Context and Problem Statement

A report surveys every session directory in a shared state tree but names rows only from this run's peer scope. A session another mount namespace launched is therefore reported under an internal directory identifier its operator has never seen.

## Considered Options

- Keep reading only this run's registry.
- Read every registry and join across all of them.
- Read the registry each session's own declared link names.

## Decision Outcome

Chosen option: read each session's declared registry. The wrapper wrote the link, so reading it is a wrapper fact, and it names one registry rather than a search.

The target's final `peers/<boot>/<namespace>` components are re-anchored under this run's peer root. The foreign absolute prefix is never opened or dereferenced. A missing link, real directory, or target outside that shape names nothing and leaves the row named by its directory.

## Consequences

- Good: sessions are named across mount boundaries without moving the boundary.
- Good: naming and reachability become separate, honest answers.
- Bad: the wrapper now reads a link it previously only wrote.
- Bad: a named foreign row still remains collectable under ADR-0112.

## Status

Implemented

Amends [ADR-0114](./ADR-0114-name-a-reported-session-as-the-child-does.md) by changing which registry a report reads. Shaped and enacted by [039](../plan/slices/039-cross-container-session-lookup/README.md).
