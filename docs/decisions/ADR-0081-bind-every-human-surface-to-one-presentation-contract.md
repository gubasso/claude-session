# ADR-0081: Bind every human surface to one presentation contract

## Context and Problem Statement

Rules about how output looks are scattered across the surfaces that first needed them: the delimiter's fixed ASCII form sits in the composed-output section, the report's bracketed status words sit in the doctor page, and the colour ladder sat in a third place. A verb author has no page that states what their human output must satisfy, so each new renderer re-decides questions the project already answered.

## Considered Options

- One presentation reference that every human renderer satisfies.
- Leave each appearance rule with the surface that first needed it.
- An explanation page describing rendering as a mental model.

## Decision Outcome

Chosen option: one presentation reference — appearance is cross-cutting, and a rule learned while building one verb has to bind the next verb, whose author has no reason to read the first verb's page.

## Consequences

- Good: a verb author reads one page before rendering anything, and a new rule has one place to land.
- Good: the rules can be checked together, because they are stated together.
- Bad: two pages now cover output, so the split between destination and appearance is a boundary that has to be maintained.
- Bad: pages that stated an appearance rule now link to it instead, which costs a reader one hop.

## Status

Accepted

The contract is [presentation](../reference/presentation.md); destinations, streams, and verbosity stay with [logging and output](../reference/logging-and-output.md). [Slice 012](../plan/slices/012-human-presentation/README.md) is the first work bound by it.
