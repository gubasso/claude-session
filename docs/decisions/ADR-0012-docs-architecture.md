# ADR-0012: Organize documentation by reader need

## Context and Problem Statement

Every durable rule this project depends on lived either inside an implementation-plan round file — an execution artifact, not a specification — or outside the repository, which [ADR-0001](./ADR-0001-self-containment.md) forbids. Writing them down needs a placement rule first, or documents accumulate wherever the author happened to be looking.

## Considered Options

- Zones by **reader need**: decisions, explanation, reference, guides — with topics inside a zone.
- A `docs/specs/` directory alongside `docs/decisions/`.
- A flat `docs/` of sibling files.

## Decision Outcome

Chosen option: **zones by reader need** — "spec" is a content category, not a reader need, and splitting on it collapses the lookup-versus-understanding distinction that makes a reference zone usable. A flat directory has no placement rule, so every future document becomes an argument.

Each zone makes one promise: decisions record why, explanation builds a mental model, reference is exact lookup, guides are ordered tasks. A topic directory goes **inside** a zone, never as a sibling. The index is an index, never a fifth zone.

Three rules govern content. **One fact, one home** — every other mention links rather than restates. **Records are lean and never deleted** — at or under 350 words in five sections, the cap acting as a splitting signal; a changed decision is superseded, a partly-changed one gains an `Amended by` line. **A zone is created by its first real document.**

Drafts live in a gitignored `.draft/`; promotion is a rewrite into the owning document, not a move.

## Consequences

- Good: placement is decided before writing, and the path tells a reader what a document is.
- Good: one home per fact means a rule is updated in one place.
- Good: the word cap keeps records reviewable and forces compound decisions to split.
- Bad: adjacent facts land in different files, so writing means linking rather than restating — the discipline most likely to erode.
- Bad: worked detail cannot live in a record, so a reader often needs a reference page too.
- Bad: never deleting records means the decisions directory only grows.

## Status

Accepted

Amended by [ADR-0041](./ADR-0041-budget-adr-length-with-a-margin.md) — the 350-word cap is a budget with a margin, measured by `wc -w`, and a record over the margin is trimmed rather than flagged. Amended by [ADR-0042](./ADR-0042-prefix-decision-filenames-with-adr.md) — a record is named `ADR-NNNN-short-title.md`.
