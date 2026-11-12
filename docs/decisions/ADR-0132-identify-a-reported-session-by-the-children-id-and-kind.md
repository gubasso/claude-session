# ADR-0132: Identify a reported session by the child's id and kind

## Context and Problem Statement

A caller that opens sessions must find each one again in `session list`. Names repeat, and several sessions often share one working directory, so the facts a row carries today do not tell those sessions apart. The child keeps its own session id and session kind in the registration the report already reads.

## Considered Options

- Leave the row as it is.
- Carry the child's session id and kind.
- Carry those two, plus the child's name provenance, version, and start time.

## Decision Outcome

Chosen option: carry the session id and the kind, under `describe-the-subject` of [ADR-0124](./ADR-0124-describe-a-reported-session-with-the-children-facts.md). The wrapper cannot derive either fact. The id is the one value unique per session that the child also shows, and the kind tells a session a person drives from a background one.

The third option fails [ADR-0051](./ADR-0051-let-every-surface-element-discriminate.md). No current use of the report discriminates by how a name was chosen or by the child's version, and the wrapper already holds a start time in its witness.

Both values pass the check the status word passes, and are carried verbatim. The wrapper does not carry the child's vocabulary of kinds. A failure that costs a row its name costs it both facts.

## Consequences

- Good: a caller finds a session by the id the child gives it.
- Good: a background session is told apart from an interactive one.
- Bad: the admitted child schema widens again, and its freshness stays tracked work.
- Bad: the kind is an ordinary word the discovery scan cannot see, so review enforces its registration. The id key is distinctive, and the gate scans it.

## Status

Implemented

Amends [ADR-0124](./ADR-0124-describe-a-reported-session-with-the-children-facts.md) with two descriptive facts, and [ADR-0114](./ADR-0114-name-a-reported-session-as-the-child-does.md) with what the registration read admits. Enacted by [`domain::registration`](../../src/domain/registration.rs). Shaped by [042](../plan/slices/042-session-list-describes-the-child/README.md).
