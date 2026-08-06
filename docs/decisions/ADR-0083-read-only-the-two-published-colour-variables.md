# ADR-0083: Read only the two published colour variables

## Context and Problem Statement

The colour ladder reads `NO_COLOR`, `FORCE_COLOR`, and `TERM`. The wider command-line convention names more inputs than that — `CLICOLOR_FORCE`, `CLICOLOR=0`, and `CLICOLOR=1` — and a common recommendation adds a continuous-integration sniff on `CI` or `GITHUB_ACTIONS` that defaults colour off. A reader holding that standard cannot tell whether this project's shorter ladder is a decision or an omission.

## Considered Options

- Read the two published variables, and record why each remaining input is rejected.
- Adopt the whole `CLICOLOR` family and the continuous-integration sniff.
- Leave the ladder as it is and say nothing, letting each reader re-derive the answer.

## Decision Outcome

Chosen option: read the two published variables — every rejected input duplicates a rung the ladder already has, so none of them can change an outcome, and a second spelling for an answered question is surface that discriminates nothing ([ADR-0051](./ADR-0051-let-every-surface-element-discriminate.md)).

## Consequences

- Good: the ladder stays four rungs, and each rung reaches an outcome no other rung does.
- Good: the rejections are recorded, so a shorter ladder reads as a decision rather than an oversight.
- Bad: a user who knows only `CLICOLOR` has to learn `NO_COLOR` to turn colour off.
- Bad: a continuous-integration runner that renders ANSI gets colour only by exporting `FORCE_COLOR`.

## Status

Accepted

The rungs and the rejected inputs are in [presentation](../reference/presentation.md#colour). `CLICOLOR_FORCE` repeats `FORCE_COLOR`, `CLICOLOR=0` repeats `NO_COLOR` more weakly, and `CLICOLOR=1` repeats the terminal test. Sniffing a continuous-integration variable repeats that test too, and would strip colour from the runners that render it. [Slice 012](../plan/slices/012-human-presentation/README.md) enacts the ladder.
