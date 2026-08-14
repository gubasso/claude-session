# ADR-0103: Permit a declared symbolic link at a wrapper-owned name

## Context and Problem Statement

Every wrapper-managed path component is refused when it is a symbolic link, reported by `storage-paths-no-symlinks`. It catches the case [ADR-0061](./ADR-0061-protect-storage-from-accidental-local-drift.md) names: a sync tool replacing a path with a link. But `CLAUDE_CONFIG_DIR` relocates every path the child reads, at once and with no finer control, so a per-terminal directory ([ADR-0102](./ADR-0102-key-child-state-by-terminal.md)) can share the projects tree back only by a link. The blanket refusal forbids the layout.

## Considered Options

- Copy the shared tree into each session directory and reconcile afterwards.
- Move session directories outside the managed region so no check applies.
- Permit a link only at a name the wrapper declares, and verify its target.

## Decision Outcome

Chosen option: permit the declared link. A link is accepted when it is the last component, the caller declared one at that exact name, and it resolves to the recorded target. Every other link is refused, and no intermediate component may be one.

This keeps the asset [ADR-0061](./ADR-0061-protect-storage-from-accidental-local-drift.md) protects. A link where none was declared still fails, because the location is wrong. A declared link that something re-pointed still fails, because the target is wrong. The rule narrows from "no link" to "no link the wrapper cannot account for", which the check was always approximating.

The wrapper never validates through a declared link: below it the child owns everything, and a walk that followed one would check a tree this project does not manage.

Copying was rejected because two writers and a reconciliation step is the predecessor's credential shape, and it drifts. Relocating storage outside the managed region gives up the checks on durable private state.

## Consequences

- Good: the wrapper can build a layout the child's all-or-nothing variable otherwise forbids.
- Bad: one more thing a check must know, and a declared name is now load-bearing.
- Bad: a user who legitimately re-points a declared link is refused rather than obeyed.

## Status

Implemented

Amends [ADR-0061](./ADR-0061-protect-storage-from-accidental-local-drift.md) and the security table in [XDG storage](../reference/xdg-storage.md). Enacted in [the storage guard](../../src/services/storage/guard.rs). Shaped by [028](../plan/slices/028-per-terminal-session-isolation/README.md).
