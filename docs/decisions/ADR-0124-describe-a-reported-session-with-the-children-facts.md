# ADR-0124: Describe a reported session with the child's facts

## Context and Problem Statement

A report can name sessions across a boundary, but two sessions of one project can carry one derived name. No wrapper-owned state holds the working directory that distinguishes them.

## Considered Options

- Leave the row at its name.
- Derive a distinguishing wrapper fact.
- Carry the child's facts about the reported subject.

## Decision Outcome

Chosen option: carry the child's facts under a fifth obligation, `describe-the-subject`: facts about a subject the report already covers, carried when the wrapper cannot derive them and the report cannot discriminate without them.

The obligation authorizes the working directory and child status word, and nothing else in the registration. ADR-0114's `name-the-subject` authorizes only the name.

## Consequences

- Good: document consumers can distinguish same-named sessions.
- Bad: the admitted child schema widens again.
- Bad: the status word is not acted on and can go stale unnoticed.
- Bad: neither fact can be registered mechanically: the status is an ordinary word, while the directory spelling has unrelated occurrences under other obligations. Both are registered here and enforced at review.

## Status

Implemented

Amends [ADR-0089](./ADR-0089-carry-a-child-owned-fact-only-against-an-obligation.md) with `describe-the-subject` and [ADR-0114](./ADR-0114-name-a-reported-session-as-the-child-does.md) with two descriptive facts. Shaped and enacted by [039](../plan/slices/039-cross-container-session-lookup/README.md).
