# ADR-0068: Spawn the child as a subroutine without adopting its answer

## Context and Problem Statement

[ADR-0005](./ADR-0005-exit-code-taxonomy.md) draws the boundary at the successful spawn: once a live child exists, the child owns the exit status. That rule was written for the passthrough launch, but three wrapper verbs also spawn the child — `account login`, `doctor`, and `version` — and read the result themselves. Taken literally the rule makes `account login` exit with `claude auth login`'s status, contradicting the account failure table, and lets the child's inherited output land on a standard output that `--json` promised to a single document.

## Considered Options

- Propagate the child's status and output from every verb that spawns one.
- Add a `ChildFailed` exit code so the origin has its own number.
- Scope the two-regimes rule to the passthrough launch, and attribute a subroutine child instead of adopting it.

## Decision Outcome

Chosen option: scope the rule to the passthrough launch. A verb that spawns the child as a subroutine keeps its own code from the matrix, because it was asked whether it provisioned an account rather than what `claude` thinks; and it keeps its own standard output, because it promised a document there.

Attribution is what replaces adoption. The diagnostic's Why opens with the child command and its status, and the JSON error document carries an optional `child_exit`, whose presence is the origin discriminator. In JSON mode a subroutine child's inherited output goes to standard error.

A new exit code was rejected: the matrix already distinguishes `Auth`, `ChildNotFound`, and `ChildNotExecutable`, and an origin is not a failure class.

## Consequences

- Good: `--json` is honest on both streams for verbs that spawn a child.
- Bad: the error document gains a field, so the one shape ADR-0032 reserved is no longer fixed.
- Bad: "the child owns the answer" now needs its scope stated wherever it is quoted.

## Status

Accepted

Amends [ADR-0005](./ADR-0005-exit-code-taxonomy.md) for the passthrough-only spawn boundary and [ADR-0032](./ADR-0032-give-each-verb-its-own-json-document.md) for optional `child_exit`.
