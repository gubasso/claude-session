# ADR-0077: Add a plan zone to reader-need documentation

## Context and Problem Statement

[ADR-0012](./ADR-0012-docs-architecture.md) organized durable documentation by reader need but left future-work status in a separate implementation queue. That queue duplicated governance and made its executor format part of the repository's operating model.

## Considered Options

- Keep four documentation zones and the separate queue.
- Add a `plan/` zone for perishable delivery state.
- Put plans beside the subsystem documents they may change.

## Decision Outcome

Choose five zones under `docs/`: `decisions/`, `explanation/`, `reference/`, `guides/`, and `plan/`. The plan zone owns the charter, milestone status, bounded vertical slices, and blocking open questions. A slice entry is its executor contract; `milestones.md` is the only delivery-status surface.

Explanation pages exist only for implemented or already-substantive models. Empty subsystem pages, plan-zone placeholders, and topic-first siblings are prohibited. ADR ids and filenames remain stable, and accepted records are never deleted.

## Consequences

- Readers can find current work through the same reader-need tree as durable knowledge.
- Perishable plan state is visibly separate from specifications and decisions.
- The old queue and its tool-specific prompt fields can be retired.
- No single page maps every binding rule to an enforcement mechanism; rules live with their owners, and mechanical rules are enforced by their actual hooks instead of a duplicate matrix.

## Status

Accepted

Supersedes [ADR-0012](./ADR-0012-docs-architecture.md).
