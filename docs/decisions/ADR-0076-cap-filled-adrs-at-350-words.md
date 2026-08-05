# ADR-0076: Cap filled ADRs at 350 words

## Context and Problem Statement

[ADR-0041](./ADR-0041-budget-adr-length-with-a-margin.md) permitted records up to 450 words while targeting 350. The margin made the target optional and left more rationale in the decision log than readers could scan quickly.

## Considered Options

- Keep the 350-word target with a 450-word margin.
- Enforce 350 words over the whole filled file.
- Measure only prose below the headings.

## Decision Outcome

Choose a hard 350-word whole-file cap for every filled ADR. `wc -w` is the deterministic measure and includes headings, links, and status text. A record that cannot fit is split into separate decisions or moves worked detail to an existing reference or explanation owner.

The cap does not justify deleting alternatives, consequences, status evidence, or relationship links. No new page is created merely to make a record shorter.

## Consequences

- Every filled record has one reproducible pass/fail measure.
- Short records keep the decision log scannable.
- Link-heavy records have less prose capacity.
- Authors must distinguish decision rationale from worked specification detail.

## Status

Accepted

Supersedes [ADR-0041](./ADR-0041-budget-adr-length-with-a-margin.md).
