# ADR-0007: One binary crate with explicit layer boundaries

## Context and Problem Statement

`claude-session` spawns processes, touches the filesystem, reads credentials, and composes configuration. Written without structure, that becomes a program where every function can do anything and none of it is testable without a real child process and a real home directory. The crate needs a shape before it has code.

## Considered Options

- A workspace of several crates, with boundaries enforced by the compiler.
- One binary crate with flat modules and no enforced layering.
- One binary crate with explicit, documented layer boundaries and adapters as the only I/O.

## Decision Outcome

Chosen option: **one crate with explicit layers** — a workspace multiplies build configuration for one consumer, and flat modules leave the program untestable.

Four roles, one prohibition each: `cli/` declares the parser and holds no logic, `commands/` orchestrates one handler per verb, `domain/` is pure, and `adapters/` is the **only** place touching the outside world, through a trait plus a real implementation. `services/` waits for a second caller.

The adapter boundary is load-bearing: a trait for the process spawner is what makes the wrapper testable without real processes. Dependencies run one way, enforced by a lint. Adding a verb touches exactly four files. See [the architecture](../explanation/architecture.md).

Workspace migration waits for a trigger: a second binary, a publishable subsystem, a slow `cargo check`, or roughly eight thousand lines.

## Consequences

- Good: the spawner, filesystem, and clock are substitutable, so most behaviour is testable without real processes or a real home directory.
- Good: one crate means one manifest, one lockfile, and a fast inner loop.
- Good: the four-edit rule makes adding a verb mechanical and reviewable.
- Bad: layer boundaries are conventions rather than compiler-enforced, so they need a lint and reviewer attention a workspace would not.
- Bad: the trait-plus-implementation pattern costs indirection at every I/O site, reading as ceremony until the first test needs it.
- Bad: deferring the workspace makes the eventual split, if it comes, a larger single change.

## Status

Accepted

Amended by [ADR-0014](./0014-xtask-workspace-for-dev-tooling.md) — the second-binary trigger has fired, so the crate gains a library target and an `xtask` workspace member. The layer rules above are unchanged.
