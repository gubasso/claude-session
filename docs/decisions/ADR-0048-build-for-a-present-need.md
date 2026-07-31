# ADR-0048: Build for a present need

## Context and Problem Statement

The wrapper is specified before it is implemented, which removes the usual brake on surface growth: nothing has to be written, so a plausible-sounding flag or subcommand costs one table row to add. The `config` verb reached six subcommands that way, several of which existed for a use nobody had — and a shipped surface is a compatibility contract, so removing one later is a breaking change.

## Considered Options

- **No rule** — judge each surface on review, case by case.
- **A documentation convention** — mention restraint in the prose guidance.
- **A binding rule with a named owner**, on the same footing as self-containment.

## Decision Outcome

Chosen option: **a binding rule** — YAGNI, stated in [`AGENTS.md` § Scope](../../AGENTS.md#scope) and mapped in [project governance](../reference/project-governance.md#rule-ownership-and-enforcement).

A flag, subcommand, configuration key, or abstraction earns its place by a use the project has today. Symmetry with a sibling, completeness of a table, and a use someone might have later are not needs. The test is a present caller.

The cost being avoided is not the writing. Every surface has to be specified to the [completeness rubric](../reference/project-governance.md), tested, documented, and kept working across every later passthrough and XDG change — and a pre-implementation project pays that cost in specification effort before it ever pays it in code.

A need that is real but not yet present is recorded as a `Rejected` option in the ADR that considered it, so it is not re-debated, rather than shipped early. That is what keeps this rule from erasing the reasoning behind a cut.

## Consequences

- Good: the surface stays small enough to specify exactly, which is what the pre-implementation model depends on.
- Good: a cut surface leaves a record, so the argument happens once.
- Bad: some genuinely useful capability arrives later than it could have, added under the change that finally needs it.
- Bad: "present need" is a judgement, so the rule is enforced by review rather than by a mechanical check.

## Status

Accepted
