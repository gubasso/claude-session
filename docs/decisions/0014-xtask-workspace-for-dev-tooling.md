# ADR-0014: An `xtask` workspace member for development tooling

## Context and Problem Statement

[ADR-0013](./0013-generate-config-examples-from-types.md) requires a generator that reflects over the wrapper's configuration types. That generator needs a schema crate and a renderer, neither of which belongs in a binary users install, and it needs to import the very types it documents — which a binary-only crate cannot export.

## Considered Options

- **A hidden verb** on the shipped binary.
- **A second `[[bin]]`** in the same crate, gated behind a feature.
- **An `xtask` workspace member**, invoked as `cargo xtask`.

## Decision Outcome

Chosen option: **an `xtask` workspace member**. [ADR-0007](./0007-layered-single-crate-architecture.md) defers a workspace until a trigger fires and names "a second binary" first among them. This is that trigger, arriving for the reason it was written down.

The crate gains a library target exporting the configuration types, and `xtask/` depends on it by path. Development-only dependencies stay in `xtask`'s manifest and never enter the shipped binary's dependency graph.

A hidden verb was rejected on two counts: it puts a development concern on a CLI surface [ADR-0003](./0003-reserve-a-small-wrapper-cli-surface.md) deliberately keeps small, and it ships schema machinery to every user. A feature-gated second binary keeps that dependency problem and adds build configuration that is easy to get wrong — a forgotten feature flag silently installs two binaries.

## Consequences

- Good: the shipped binary carries no generator weight and no schema dependency.
- Good: `cargo xtask` is a conventional, discoverable home for future repository chores.
- Good: the library target makes configuration types reachable from integration tests too.
- Bad: the repository becomes a two-member workspace, so build configuration doubles.
- Bad: a library target invites re-exporting more than intended; only what the tooling needs should be public.

## Status

Accepted

Amends [ADR-0007](./0007-layered-single-crate-architecture.md) — the workspace trigger it defers has fired.
