# ADR-0094: Give every check a title and a next action

## Context and Problem Statement

A catalog entry carries an id, a scope, a severity, an error kind, and one remediation template. Nothing in it says, in words, what the check protects — so the human report has only the id to lead a row with, and an inapplicable check has only a bare reason. [ADR-0093](./ADR-0093-write-every-non-machine-surface-for-a-person.md) requires a consequence and a next action; the catalog cannot supply either.

## Considered Options

- Give each entry a title and a consequence beside its remediation, all owned by the catalog.
- Keep the wording in the renderer, matching on the check to decide what to print.
- Derive a title from the id by unhyphenating it.

## Decision Outcome

Chosen option: own the title and the consequence in the catalog — the verbatim rule that puts one remediation beside its check ([ADR-0018](./ADR-0018-one-probe-set-with-stable-check-ids.md)) applies unchanged to the two strings a reader needs before it, and a guard quoting a check must be able to quote them too.

## Consequences

- Good: a guard and a report say the same thing about the same condition, in the same three parts.
- Good: adding a check forces its author to say what it protects, in the same edit that names it.
- Bad: the catalog grows two strings per entry and the published table grows a column.
- Bad: a title is a second name for a check, and a careless one will drift from the id it sits beside.

## Status

Implemented

The strings live beside `id`, `scope`, `severity`, and `remediation` in the catalog, and are published in [the doctor catalog table](../reference/doctor.md#the-catalog). Titles are not identifiers: they are absent from `--json` and free to be reworded, which is what keeps [ADR-0018](./ADR-0018-one-probe-set-with-stable-check-ids.md)'s breaking-change rule attached to the id alone. [Slice 020](../plan/slices/020-human-first-diagnostics/README.md) enacts it.
