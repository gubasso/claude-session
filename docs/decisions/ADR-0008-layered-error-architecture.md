# ADR-0008: Typed errors per layer, boundary errors only at the edge

## Context and Problem Statement

[ADR-0005](./ADR-0005-exit-code-taxonomy.md) requires every wrapper failure to map to exactly one exit code, exhaustively and with no catch-all. That is only possible if the error type reaching the entry point is a closed set, and the ergonomic alternative — one opaque error type everywhere — erases exactly the information the mapping needs.

## Considered Options

- One opaque boundary error type throughout, with the exit code derived by downcast.
- Typed enums throughout, including the entry point.
- Typed enums per layer converging on one application error, with the boundary type confined to the entry point.

## Decision Outcome

Chosen option: typed per layer, boundary type only at the edge — typed errors give the closed enum the exit-code matrix needs, while the boundary type stays the right tool for assembling a final report.

Each layer has its own error: domain invariants, one per adapter, service orchestration. They converge on one application error — a closed enum with no catch-all variant — owning the exit-code mapping. The boundary type appears only in the entry point, and a boxed trait-object error is never a return type in this crate.

Two rules keep it useful rather than ceremonial. Every error carries the concrete value involved — the path, the key, the account — because an error that cannot say which file failed has failed at its job. And a cross-layer conversion either derives or is hand-written to add context. See [coding conventions](../reference/coding-conventions.md).

## Consequences

- Good: the exit-code mapping is exhaustive by construction, so an unmapped variant fails the build.
- Good: errors carry their concrete subject, which makes the four-part message shape possible.
- Bad: more types and conversions than one opaque error, with boilerplate in every new adapter.
- Bad: adding a variant means touching the enum, the mapping, and the matrix test — deliberate friction, since each addition changes a public interface.

## Status

Accepted

Extended by [ADR-0035](./ADR-0035-convert-the-typed-error-to-a-code-once.md) — how the closed enum becomes a process status.
