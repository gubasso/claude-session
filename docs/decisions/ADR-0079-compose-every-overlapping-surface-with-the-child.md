# ADR-0079: Compose every overlapping surface with the child

## Context and Problem Statement

[ADR-0045](./ADR-0045-compose-doctor-with-the-child-report.md) lets a verb keep a colliding name when it runs the child's command as part of its own, and [ADR-0030](./ADR-0030-use-account-login-for-wrapper-authentication.md) renamed `auth` on reasoning of its own. Neither says when composing is allowed, so the next overlap is argued from scratch. Meanwhile `--help` claims a colliding spelling while describing only the wrapper's grammar, though [ADR-0044](./ADR-0044-audit-wrapper-spellings-against-the-child-inventory.md) keeps it as a superset of the child's.

## Considered Options

- Compose every overlap whose shared surface is read-only, and rename the rest.
- Keep resolving each overlap on its own reasoning, as ADR-0030 and ADR-0045 did.
- Rename every overlap, so no wrapper surface spawns the child in order to print.

## Decision Outcome

Chosen option: compose read-only overlaps — a user who typed a colliding name wants both answers, and the read-only test names the overlaps that can give both.

Three rules bind flags and verbs alike. A claimed spelling the child also owns is kept only when the shared surface is read-only: checks, reports, help, version. A surface that runs the agent, opens a terminal interface, or changes state is renamed instead, since composing there would perform the effect twice. A kept spelling emits the wrapper's output first, then the child's bytes verbatim under the delimiter [logging and output](../reference/logging-and-output.md#composed-output) owns.

The child runs as a subroutine ([ADR-0068](./ADR-0068-spawn-the-child-as-a-subroutine.md)) and is never parsed. An unresolvable child replaces its section with that condition and leaves the exit unchanged. Overlap stays measured rather than assumed ([ADR-0044](./ADR-0044-audit-wrapper-spellings-against-the-child-inventory.md)), so a claimed name the child does not own composes nothing.

## Consequences

- Good: one criterion predicts `doctor`, `auth`, `--help`, and `--version`, so a new overlap is a lookup.
- Good: no claimed spelling hides an answer the child would have given.
- Bad: `--help` now spawns the child, so it is slower and no longer self-contained.
- Bad: the child's formatting is outside the wrapper's control on one more surface.

## Status

Accepted

Amends [ADR-0003](./ADR-0003-reserve-a-small-wrapper-cli-surface.md), [ADR-0030](./ADR-0030-use-account-login-for-wrapper-authentication.md), and [ADR-0045](./ADR-0045-compose-doctor-with-the-child-report.md). It relies on [ADR-0044](./ADR-0044-audit-wrapper-spellings-against-the-child-inventory.md) unchanged: that audit decides what overlaps, this record how.
