# ADR-0024: Machine output is a per-verb `--json` flag

## Context and Problem Statement

[ADR-0017](./ADR-0017-declare-a-human-facing-cli.md) settled that machine-readable output is opt-in, but not how the user asks for it. The wrapper had claimed a global `--format <text|json>` on the wrapper-owned flag table, which is the denylist of flags intercepted before the passthrough split.

That placement contradicts a rule the same page already states: a top-level flag is subtracted from the child's reachable surface permanently, so the wrapper claims one only when it must. Machine output has no meaning for a passthrough invocation — the wrapper writes nothing to standard output during one — so the flag was costing the child a name in exchange for nothing.

## Considered Options

- **Global `--format <text|json>`** — one flag, on the denylist, before the verb.
- **Per-verb `--json`** — each verb that produces data declares its own, after the verb.
- **Infer from whether standard output is a terminal** — already rejected by ADR-0017.

## Decision Outcome

Chosen option: **per-verb `--json`**. It costs the child nothing, because a flag parsed after a wrapper verb sits inside an invocation the child never sees — the same reasoning that already keeps `--yes` and `doctor --list` off the table.

It also keeps each verb's output schema independent. A global format flag implies one output contract; declaring the flag per verb makes explicit that `account list --json` and `doctor --json` are separate documents that may evolve separately.

The format contract is unchanged and still owned by [logging and output](../reference/logging-and-output.md#machine-output): one JSON document on standard output, no human-oriented text in that mode, errors on standard error carrying `err.kind`. One output writer still renders every document.

## Consequences

- Good: the wrapper-owned flag table shrinks by one, widening the child's reachable surface.
- Good: `--json` is the spelling most CLI users expect, and needs no value.
- Bad: a future second format (`--yaml`) is a new flag per verb rather than a new value on one.
- Bad: every data verb must remember to declare the flag; the [development workflow](../guides/development-workflow.md) checklist carries the reminder.

## Status

Accepted
