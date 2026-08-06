# ADR-0003: Reserve a small wrapper CLI surface

## Context and Problem Statement

Given verbatim passthrough ([ADR-0002](./ADR-0002-verbatim-argv-passthrough.md)), every flag and verb the wrapper claims is one the child can no longer receive. It still needs its own surface — accounts, profiles, diagnostics — so the question is how much argument space to take, and how to structure it.

## Considered Options

- A small, closed, top-level set of long-form flags and verbs, documented as a denylist.
- Namespace every wrapper command under a single reserved verb, leaving the rest of the surface free.
- Mirror the child's grammar and extend it, so wrapper and child options interleave naturally.

## Decision Outcome

Chosen option: a small, closed, top-level set — nesting costs a token on every invocation to solve a collision problem a short documented list already solves, and mirroring the child's grammar is the coupling [ADR-0002](./ADR-0002-verbatim-argv-passthrough.md) rejects.

Three rules make the surface safe. Wrapper flags are long-form except where a short form is universal, so short flags stay available to the child. Wrapper flags appear before the verb, never after, and never after `--`. The claimed set is enumerated with a reason per entry in [the CLI surface](../reference/cli-surface.md).

## Consequences

- Good: the collision surface is small, explicit, and reviewable rather than implicit.
- Good: `--` is an unconditional escape to any claimed name.
- Bad: adding a flag to the set removes it from the child's reachable surface, so growing the set is a passthrough-contract change needing an amendment here, not a table edit.
- Bad: if the child adds a subcommand matching a wrapper verb, the wrapper wins and users need `--`; the collision is recorded rather than silently resolved.

## Status

Accepted

Amended by [ADR-0015](./ADR-0015-retire-the-init-verb.md) (`init` leaves the claimed verb set), [ADR-0016](./ADR-0016-ship-man-pages.md) (`man` joins it), [ADR-0043](./ADR-0043-match-wrapper-flags-by-exact-leading-spelling.md) (how a token is recognized as claimed), [ADR-0044](./ADR-0044-audit-wrapper-spellings-against-the-child-inventory.md) (`-v` leaves the set, and collisions are audited rather than recorded), [ADR-0045](./ADR-0045-compose-doctor-with-the-child-report.md) (a verb may keep a colliding name when it composes with the child's), and [ADR-0079](./ADR-0079-compose-every-overlapping-surface-with-the-child.md) (a read-only overlap composes; every other is renamed).
