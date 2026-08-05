# ADR-0032: Give each verb's JSON document its own shape

## Context and Problem Statement

[ADR-0024](./ADR-0024-machine-output-is-a-per-verb-flag.md) made `--json` verb-level but said nothing about what the document looks like. `doctor` needs a catalog with counts and a version; `version` needs two fields. Deciding this once matters because the shape is what scripts match on, and widening it later is a breaking change.

## Considered Options

- One envelope for every verb — `{status, data, error}`, with the verb's payload nested under `data`.
- A bare, verb-owned top-level object, with shared field-level rules.
- No rule — settle each verb's shape as it is implemented.

## Decision Outcome

Chosen option: a bare, verb-owned top-level object — a shared envelope is a contract every verb must honour forever to buy uniformity that no consumer of a single verb needs.

An envelope's usual payoff is a uniform error path, and this wrapper already has one: failures exit non-zero and write an `err.kind` to standard error, so a script branches on the exit status before it ever parses standard output. That leaves the envelope costing every verb two levels of nesting for nothing.

Three field-level rules stay shared, since they are cheap and their absence is what makes documents awkward to consume: one document per invocation, absent optional fields omitted rather than `null`, and `schema_version` only where the document is itself a matched-against contract.

The error document is the exception and is uniform across verbs, because a caller handling failure is handling the one case that is not verb-specific. The shapes are in [logging and output](../reference/logging-and-output.md#machine-output).

## Consequences

- Good: `doctor --json | jq '.checks[]'` needs no unwrapping step.
- Bad: a generic consumer of several verbs writes per-verb handling; there is no single shape to program against.
- Bad: `schema_version` on one verb and not others is an inconsistency that has to be explained each time it is noticed.

## Status

Accepted

Amended by [ADR-0068](./ADR-0068-spawn-the-child-as-a-subroutine.md) — the shared error document gains one optional field, `child_exit`, present only when a spawned child produced the failure.
