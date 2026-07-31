# ADR-0015: Retire the `init` verb

## Context and Problem Statement

`init` was specified to create the wrapper's configuration scaffold. [ADR-0013](./ADR-0013-generate-config-examples-from-types.md) removes that job: users copy a generated example instead. The question left is whether the verb survives with a narrower purpose or goes.

## Considered Options

- **Keep it for state and cache** — create directories, prime a registry, print where configuration would go.
- **Keep it as a binding assistant** — read-only by default, persisting only under an explicit `--write`.
- **Remove it.**

## Decision Outcome

Chosen option: **remove it** — nothing is left for it to do.

Every surviving purpose already has an owner. Configuration is optional, because every key has a compiled-in default, so there is no first-run setup to perform. Credentials belong to `account`. Session directories are created on demand by the run that needs them.

The binding-assistant pattern answers a problem this wrapper does not have. It exists to record a persistent project-to-profile binding in state; here the active profile is a configuration key and a `--profile` flag, resolved fresh on every invocation, with nothing to persist between runs.

A verb kept for a job it no longer has is worse than no verb. It occupies a name the child may one day want, and it teaches users to expect a setup step the tool does not need.

## Consequences

- Good: one fewer name claimed from the child's reachable surface.
- Good: the first-run experience is "run it" — configuration is purely optional.
- Bad: users arriving from tools with an `init` convention will look for one; `doctor` and the generated examples have to carry that weight.
- Bad: if a future subsystem needs real setup, the verb must be reintroduced by amending this record rather than by editing a table.

## Status

Accepted

Amends [ADR-0003](./ADR-0003-reserve-a-small-wrapper-cli-surface.md) — the claimed verb set shrinks by one.
