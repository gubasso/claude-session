# ADR-0015: Retire the `init` verb

## Context and Problem Statement

`init` was specified to create the wrapper's configuration scaffold. [ADR-0013](./ADR-0013-generate-config-examples-from-types.md) removes that job: users copy a generated example instead. The question left is whether the verb survives with a narrower purpose or goes.

## Considered Options

- Keep it for state and cache — create directories, prime a registry, print where configuration would go.
- Keep it as a binding assistant — read-only by default, persisting only under an explicit `--write`.
- Remove it.

## Decision Outcome

Chosen option: remove it — nothing is left for it to do.

Every surviving purpose already has an owner. The wrapper does not require a scaffold: selections may come from flags, environment, user configuration, or the account marker. Credentials belong to `account`, profiles start from shipped examples, and session directories are created on demand by a bound run.

The binding-assistant pattern answers a problem this wrapper does not have. It exists to record a persistent project-to-profile binding in state; here the active profile is a configuration key and a `--profile` flag, resolved fresh on every invocation, with nothing to persist between runs.

A verb kept for a job it no longer has is worse than no verb. It occupies a name the child may one day want, and it teaches users to expect a setup step the tool does not need.

## Consequences

- Good: one fewer name claimed from the child's reachable surface.
- Good: first-run recovery uses `account login` and shipped examples instead of another setup surface.
- Bad: users arriving from tools with an `init` convention will look for one; `doctor` and the generated examples have to carry that weight.
- Bad: if a future subsystem needs real setup, the verb must be reintroduced by amending this record rather than by editing a table.

## Status

Accepted

Amends [ADR-0003](./ADR-0003-reserve-a-small-wrapper-cli-surface.md) — the claimed verb set shrinks by one.

Amended by [ADR-0091](./ADR-0091-refuse-an-unconfigured-first-launch-without-scaffolding.md), which keeps the no-`init` decision while replacing unconfigured first-launch success with actionable refusal.
