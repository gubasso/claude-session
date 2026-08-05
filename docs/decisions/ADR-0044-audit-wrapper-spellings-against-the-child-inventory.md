# ADR-0044: Audit wrapper spellings against the child inventory

## Context and Problem Statement

[ADR-0003](./ADR-0003-reserve-a-small-wrapper-cli-surface.md) says a collision is "recorded rather than silently resolved", but nothing ever compared the claimed set against the child. Measured against `claude` 2.1.220 on 2026-07-31, two claims collide: `--verbose` is a native flag, and `-v` is the child's `--version`. Recording is too weak a remedy for a project whose first contract is never breaking passthrough.

## Considered Options

- Audit the claimed set against a recorded child inventory, and block release on an undocumented intersection.
- Keep recording collisions in prose as they are noticed.
- Let the child win, resolving every claimed name against the child at run time.

## Decision Outcome

Chosen option: audit and block — a collision is a fact about two programs, so it is checked mechanically or not at all.

Three rules. A wrapper flag is subtracted from the child only in leading position; after the first non-wrapper token, or after `--`, the child's spelling is reachable unchanged. Every intersection between the claimed set and the child's inventory is named in the CLI surface with the child's meaning. An intersection that is not named there fails the build, against a checked-in, version-labelled inventory fixture.

`-v` is dropped from the claimed set. Every other collision merely shadows a child spelling the escape hatch still reaches; `-v` would change a token's meaning, which no escape hatch repairs. `--help`, `-h`, and `--version` stay as named exceptions, since the wrapper's answer is a superset of the child's.

Letting the child win was rejected: resolving names at run time requires the model of the child's grammar [ADR-0002](./ADR-0002-verbatim-argv-passthrough.md) forbids.

## Consequences

- Good: the collision claim becomes a test rather than a promise, and fails the day the child grows `--dry-run`.
- Good: the fixture dates every child fact, so a stale claim is visible.
- Bad: a child release can turn a green build red without any wrapper change.
- Bad: verbosity loses its short form.

See [the CLI surface](../reference/cli-surface.md#when-the-child-owns-the-same-name) and the `child-flag-and-verb-inventory` fact in [research tracking](../reference/research-tracking.yaml).

## Status

Accepted

Amends [ADR-0003](./ADR-0003-reserve-a-small-wrapper-cli-surface.md).
