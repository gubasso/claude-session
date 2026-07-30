# ADR-0009: A reviewed dependency set, added only with cargo add

## Context and Problem Statement

Dependencies are the hardest thing to back out of: they shape the code using them, carry audit and licence obligations, and pull in a transitive tree a later change cannot undo cheaply. A wrapper that spawns one process and composes some JSON does not need a large tree, so the set deserves deciding once.

## Considered Options

- Hand-roll everything but the argument parser, keeping the tree minimal.
- Maintain a reviewed candidate list, adding a crate only when a specific piece of work needs it.
- Add whatever is convenient at the point of need, governed only by supply-chain gates.

## Decision Outcome

Chosen option: **a reviewed candidate list, added on demand** — hand-rolling signal handling, XDG resolution, and layered configuration reimplements well-tested crates badly, while an unconstrained set accumulates weight nobody chose.

The list is **candidates, not decisions**: being on it does not put a crate in the manifest, and a crate is added when work needs it and not before. An unused dependency is compile time, audit surface, and supply-chain risk bought for nothing. See [dependencies](../reference/dependencies.md).

Two mechanical rules. Dependencies are **always added with `cargo add`**, never by hand-editing the manifest and never by writing a version string, so the graph resolves and the lockfile updates in one step; deviating needs a documented reason. And `Cargo.lock` is **committed**, because this is a binary and reproducible builds are the point.

## Consequences

- Good: the tree stays proportionate, and every crate in it has a recorded reason.
- Good: `cargo add` keeps manifest and lockfile consistent, avoiding hand-written versions that are stale or over-tight the day they are written.
- Good: the reference page carries **no version numbers**, so it cannot go stale — versions live in the lockfile.
- Bad: the ruled-out list needs maintaining when a crate is deprecated, which is why it is a tracked perishable fact.
- Bad: adding anything not on the list requires a recorded assessment — intended friction, but friction.
- Bad: a committed lockfile means dependency-update noise in the history, and keeping it current is manual work ([ADR-0023](./0023-only-release-automation-opens-pull-requests.md)).

## Status

Accepted
