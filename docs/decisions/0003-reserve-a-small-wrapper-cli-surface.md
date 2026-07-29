# ADR-0003: Reserve a small wrapper CLI surface

## Context and Problem Statement

Given verbatim passthrough ([ADR-0002](./0002-verbatim-argv-passthrough.md)), every flag and verb the wrapper claims is one the child can no longer receive. The wrapper still needs its own surface — accounts, profiles, diagnostics — so the question is how much of the argument space to take and how to structure it.

## Considered Options

- A small, closed, top-level set of long-form flags and verbs, documented as a denylist.
- Namespace every wrapper command under a single reserved verb, leaving the rest of the surface free.
- Mirror the child's grammar and extend it, so wrapper and child options interleave naturally.

## Decision Outcome

Chosen option: **a small, closed, top-level set** — nesting costs a token on every wrapper invocation to solve a collision problem that a short documented list already solves, and mirroring the child's grammar is exactly the coupling [ADR-0002](./0002-verbatim-argv-passthrough.md) rejects.

Three rules make the surface safe. Wrapper flags are **long-form** except where a short form is universal, so short flags stay available to the child. Wrapper flags appear **before** the verb, never after, and never after `--`. The claimed set is **enumerated with a reason per entry** in [the CLI surface](../reference/cli-surface.md), so a reader can see the whole collision surface on one screen.

## Consequences

- Good: wrapper commands are as short to type as the child's, which matters for a tool run dozens of times a day.
- Good: the collision surface is small, explicit, and reviewable rather than implicit.
- Good: `--` gives the user an unconditional escape to any claimed name.
- Bad: adding a flag to the set removes it from the child's reachable surface, so growing the surface is a passthrough-contract change requiring an amendment here — not merely a table edit.
- Bad: if the child ever adds a subcommand matching a wrapper verb, the wrapper wins and users need `--`. The collision would be recorded rather than silently resolved.
- Bad: help and completions describe only the wrapper's grammar, so users must consult the child for its own flags.

## Status

Accepted

Amended by [ADR-0015](./0015-retire-the-init-verb.md) — `init` leaves the claimed verb set — and by [ADR-0016](./0016-ship-man-pages.md) — `man` joins it.
