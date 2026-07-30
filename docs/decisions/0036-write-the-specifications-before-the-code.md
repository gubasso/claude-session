# ADR-0036: Write the specifications before the code

## Context and Problem Statement

This project authors its documentation ahead of its implementation — the specifications under `docs/` are what the queued rounds are built from. [ADR-0022](./0022-cut-the-first-release-when-passthrough-works.md) drew the opposite conclusion for one zone, that a guide cannot honestly be written before `0.1.0` exists, which held `guides/` at a single page and made an unwritten guide look like a decision rather than a gap.

## Considered Options

- Keep the pre-release cap: guides wait until a user can run the binary.
- Cap nothing, but write a document only once there is code to describe.
- Write whatever the implementation needs, in whatever volume, before the code.

## Decision Outcome

Chosen option: **write whatever the implementation needs, before the code**. A specification here is an input to implementation, not a report on it, so the reader served first is the implementer — and that reader is blocked by an absent document, never by an unshipped binary.

No zone has a page cap. A guide is written when a task needs an ordered procedure, and it is verified against the specification and the gate rather than against a release. The rules that already bound documents still bind: one fact one home, a zone created by its first real document, and never a placeholder ([ADR-0012](./0012-docs-architecture.md)).

## Consequences

- Good: the specification set is sized by what implementation requires, not by what a release permits.
- Good: a thin zone reads as work outstanding, which is what it is.
- Bad: a guide written ahead of its code can drift from it, so each is re-checked when its subject is implemented.
- Bad: more documents to hold consistent, and one fact one home is the discipline that pays for it.

## Status

Accepted

Amends [ADR-0022](./0022-cut-the-first-release-when-passthrough-works.md), whose release decision is untouched.
