# ADR-0082: Colour a closed set of named surfaces

## Context and Problem Statement

The colour ladder is specified, implemented as a pure function, and unit tested, but its result is discarded where the context is built, and no byte the wrapper writes carries an escape sequence. The reference, the code, and the tests agree with each other and with nothing a user sees, so the tests would stay green if the ladder were deleted outright.

## Considered Options

- Colour a closed set of named surfaces, and assert the escape bytes.
- Delete the ladder from the code and the reference together.
- Colour whatever a renderer judges useful, under the existing ladder.

## Decision Outcome

Chosen option: colour a closed set of named surfaces — the ladder already encodes what a terminal user expects, and naming the surfaces keeps colour reviewable in one place, where an open licence would push the question back into every renderer.

## Consequences

- Good: the ladder reaches bytes, so a test asserting their presence or absence can fail.
- Good: colour stays decoration, because every named surface states its meaning in text regardless.
- Bad: a presentation crate enters the shipped graph to serve three surfaces.
- Bad: adding a fourth surface costs an edit to an owning page rather than a renderer's judgement.

## Status

Accepted

The ladder and the surfaces are in [presentation](../reference/presentation.md), and [slice 012](../plan/slices/012-human-presentation/README.md) enacts them. This resolves the colour question raised by [slice 011](../plan/slices/011-entry-point-and-contract-repair/README.md), which found the value discarded and left both the code and the specification alone.
