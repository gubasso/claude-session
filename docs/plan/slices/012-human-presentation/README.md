# 012 — Human presentation

## Goal

Decide and document the presentation contract, so the renderer that later applies it has one binding specification rather than a per-verb judgement.

## Appetite

2 implementation sessions.

## Core

One owner states the colour ladder and the closed set of colourful surfaces, and the resolver already in the code agrees with it.

## In scope

- The presentation contract as one reference page binding every human renderer, current and later.
- The colour ladder as a first-match order over the two published variables, including the empty-value half of each convention.
- The inputs the ladder deliberately does not read, stated rather than left to be inferred.
- The closed set of named colourful surfaces: the diagnostic kind, the mirrored level word, and the composed delimiter.
- The decision records behind the single contract, the closed surface set, and the two-variable ladder.
- The colour resolver corrected to the published order and resolved once per destination on the immutable context, under unit coverage, with its absence from every stream provable.

## Out of scope

- Colouring a surface, which each renderer does under the standing rule rather than in a slice of its own.
- A presentation crate, which the first coloured surface admits.
- Tables, progress indicators, spinners, and interactive prompts.
- A wrapper flag for colour, which the passthrough contract forbids.
- The doctor report renderer, which slice 007 owns.
- Any new verb, document, or output surface.

## Governed by

- [AGENTS.md](../../../../AGENTS.md)
- [Presentation](../../../reference/presentation.md)
- [Logging and output](../../../reference/logging-and-output.md)
- [CLI surface](../../../reference/cli-surface.md)
- [Coding conventions](../../../reference/coding-conventions.md)
- [Dependencies](../../../reference/dependencies.md)
- [Testing and quality](../../../reference/testing-and-quality.md)
- [ADR-0017](../../../decisions/ADR-0017-declare-a-human-facing-cli.md)
- [ADR-0081](../../../decisions/ADR-0081-bind-every-human-surface-to-one-presentation-contract.md)
- [ADR-0082](../../../decisions/ADR-0082-colour-a-closed-set-of-named-surfaces.md)
- [ADR-0083](../../../decisions/ADR-0083-read-only-the-two-published-colour-variables.md)

## Acceptance

- Where `FORCE_COLOR` is present and not empty, the wrapper shall resolve colour on whatever the destination is. -> ui::writer::tests::forced_color_wins_over_denied_color
- Where `NO_COLOR` or `FORCE_COLOR` is present and empty, the wrapper shall treat that variable as unset. -> ui::writer::tests::an_empty_override_is_inert
- Where no override is active and the destination is a dumb terminal or not a terminal, the wrapper shall resolve colour off. -> ui::writer::tests::a_dumb_or_redirected_destination_fails_closed
- While the output mode is JSON, the wrapper shall resolve colour off on both streams whatever the environment asked for. -> ui::writer::tests::machine_mode_dominates_every_override
- The wrapper shall resolve the decision once per destination, so one redirected stream does not decide the other's appearance. -> ui::writer::tests::each_destination_resolves_its_own_terminal_rung
- Where a renderer applies the resolved decision, the plain bytes shall equal the decorated ones with the escapes stripped, and no machine document shall carry one. -> output::colour_decorates_the_diagnostic_without_changing_it
- The presentation reference shall state the ladder, the inputs it does not read, and the closed surface set as the single owner of each.

## Rabbit holes

- Building a palette or a theme; escape: name the three surfaces and stop.
- Adding a `--color` flag to settle a precedence argument; escape: the ladder is environment-only and the spelling belongs to the child.
- Writing a renderer to prove the specification; escape: the verb that needs the surface writes it, and a spec proved by its own first implementation is a spec nothing checked.
- Reaching for a terminal-control crate to reuse its detection; escape: the ladder is four rungs over an environment snapshot.

## Done when

The presentation reference and its three decision records state the contract, the resolver matches the published ladder under unit coverage, no wrapper surface emits an escape byte, and `just hooks` is green.

## Revisions

- 2026-08-07: `Goal`, `Core`, and `Acceptance` narrowed from carrying colour to bytes to deciding and documenting the contract. The specification was the deliverable the work actually needed: the ladder's precedence was written two ways before it settled, and a renderer built against either draft would have shipped the wrong order under a test that agreed with it. Applying the decision is not a slice: colour binds every renderer the project will write, so it lands as a standing rule that each verb satisfies when it renders its surface.
- 2026-08-07: The decision is one value per destination rather than one boolean, and machine mode short-circuits the ladder rather than sitting inside it. Carrying a single answer derived from standard output would have coloured the diagnostic, which lands on standard error, by asking the wrong stream whether it was a terminal — the ordinary case of a piped command.
- 2026-08-07: The resolver's own order was corrected here rather than deferred. It read presence without emptiness and put deny above force, so the code contradicted the page that owns it, and leaving a wrong rule under a passing test is worse than leaving no rule at all.
