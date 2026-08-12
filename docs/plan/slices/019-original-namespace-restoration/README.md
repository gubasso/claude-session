# 019 — Original namespace restoration

## Goal

Return every name 018 moved to `claude-session`, once the shell predecessor is deprecated and nothing is left to collide with, so the coexistence spelling does not outlive the coexistence.

## Appetite

1 implementation session.

## Core

Every name 018 moved is back, and the record that moved it is closed rather than left standing.

## In scope

- The four XDG namespace directories, the log filename, the project configuration filename, the command name, and the environment prefix, all reversing 018 at the same call sites.
- A migration for state the wrapper wrote under the coexistence namespace, since by then it holds real accounts and composed entries rather than the empty directories 018 started from.
- Supersession or deprecation of 018's decision, whichever its own text calls for, so no current record still asserts a namespace the tree does not use.
- Alignment of the same owners 018 moved, and of the generated examples that print a destination path.

## Out of scope

- Removing the predecessor, which is not this project's to uninstall and is the precondition rather than the work.
- Any behaviour change riding along with the rename; a name moves or it does not.
- Reading the coexistence namespace at runtime after the migration, which would make the compatibility permanent in the name of ending it.

## Governed by

- [AGENTS.md](../../../../AGENTS.md)
- [Charter](../../charter.md)
- [ADR-0006](../../../decisions/ADR-0006-place-files-by-xdg-ownership.md)
- [ADR-0057](../../../decisions/ADR-0057-build-the-child-environment-by-prefix-scrub-and-marker.md)
- [ADR-0070](../../../decisions/ADR-0070-discover-the-project-configuration-file-at-the-repository-root.md)
- [XDG storage](../../../reference/xdg-storage.md)
- [Configuration](../../../reference/configuration.md)
- [CLI surface](../../../reference/cli-surface.md)
- [Coding conventions](../../../reference/coding-conventions.md)
- [Testing and quality](../../../reference/testing-and-quality.md)
- [Development workflow](../../../guides/development-workflow.md)

## Acceptance

- When the wrapper resolves a path under any XDG base, the namespace directory shall be `claude-session`.
- When the wrapper builds the child environment, it shall scrub the original prefix.
- Where a document or generated example names a path or variable the wrapper reads or writes, it shall name the original spelling and no other.
- When the work lands, no current record shall assert the coexistence namespace.

## Rabbit holes

- Starting before the predecessor is actually gone, which reinstates every collision 018 removed; escape: the entry condition is deprecation, not confidence.
- Building a general namespace-migration facility for a move that happens once; escape: one migration, deleted with the slice.

## Done when

Every name is back, the state the wrapper wrote under the coexistence namespace has moved with it, 018's decision is no longer current, and `just hooks` is green.

## Revisions

None.
