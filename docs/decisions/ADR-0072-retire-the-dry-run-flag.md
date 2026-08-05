# ADR-0072: Retire the `--dry-run` flag

## Context and Problem Statement

`--dry-run` sits on the wrapper's denylist, which permanently subtracts that spelling from the child's reachable surface. Nothing else in the repository specifies it: no output document, no exit regime, no `--json` shape, no mandatory test. It is a claimed flag with no contract behind it, and the denylist-membership test would assert a row that means nothing.

## Considered Options

- Specify it — give it an output document, an exit regime, and a test.
- Retire it — remove the row and return the spelling to the child.
- Leave it claimed and unspecified.

## Decision Outcome

Chosen option: retire it — `config` already answers the question a rehearsal would ask, so the flag discriminates nothing while costing the child a spelling.

`config` resolves the active profile, the pieces it names, the composed entry path, provenance, freshness, and defects, and it spawns nothing ([ADR-0049](./ADR-0049-collapse-config-inspection-into-one-verb.md)). A rehearsal flag reporting those same facts would be a second surface for one answer, which [ADR-0051](./ADR-0051-let-every-surface-element-discriminate.md) rejects. Specifying it first and finding the consumer later inverts the order [ADR-0048](./ADR-0048-build-for-a-present-need.md) requires.

Re-add it only when a rehearsal answers something `config` cannot — the exact argument vector the wrapper would hand the child, say — and then under its own record.

## Consequences

- Good: the child regains `--dry-run`, and the denylist becomes a table where every row has a contract behind it.
- Good: the denylist-membership test gains meaning, since the documented set is now exactly the set with specified behaviour.
- Bad: removing a documented flag is a change to the passthrough surface, which is why it takes a record rather than an edit.
- Bad: a user who read the flag table and expected a rehearsal has to reach for `config` instead.

## Status

Accepted

Removes the `--dry-run` row from [the CLI surface](../reference/cli-surface.md#wrapper-owned-flags) and its mention from [logging and output](../reference/logging-and-output.md#log-records). Narrows the surface [ADR-0003](./ADR-0003-reserve-a-small-wrapper-cli-surface.md) reserved.
