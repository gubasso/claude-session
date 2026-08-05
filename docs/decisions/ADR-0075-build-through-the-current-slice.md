# ADR-0075: Build through the current slice

## Context and Problem Statement

[ADR-0036](./ADR-0036-write-the-specifications-before-the-code.md) allowed unlimited specification work ahead of implementation. That made completeness open-ended and let future design expand without a delivery boundary.

## Considered Options

- Continue specifying every anticipated subsystem before implementation.
- Bound new specifications and decisions to the current slice.
- Stop maintaining specifications until code exists.

## Decision Outcome

Choose the current-slice gate. After the documentation refactor that introduces this rule is complete, no specification page or ADR outside the current slice may be opened until that slice is implemented. Unsettled work goes to [open questions](../plan/open-questions.md); the next slice may shape its needed documents when it becomes current.

The existing technical contracts remain authoritative. The gate controls new work rather than deleting specifications that already constrain the product.

## Consequences

- Delivery has one bounded design frontier and one visible status surface.
- A slice may still revise its own goal, scope, acceptance, and governing documents while active.
- Later work stays deliberately less detailed until it becomes current.
- The completeness rubric retired with ADR-0036's model. Specification completeness is now bounded by the current slice's `Acceptance`, not by an open-ended rubric.

## Status

Accepted

Supersedes [ADR-0036](./ADR-0036-write-the-specifications-before-the-code.md).
