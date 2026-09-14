# ADR-0125: Supply skills from the child's own directory

## Context and Problem Statement

[ADR-0106](./ADR-0106-supply-child-assets-from-one-tree.md) supplies ten assets from one wrapper-owned tree, which makes that tree the anchor and `~/.claude/skills` a link into it. Skills are the only one of the ten with installers: a project installs its own skills, and an installer writes the directory the child publishes. One that refuses to write through a symbolic link fails against the anchor, and one that follows it writes a tree the wrapper claims to own. The other nine assets are hand-authored and have no such writer.

## Considered Options

- Keep skills in the wrapper tree, and ask every installer to target it.
- Keep the anchor, and let each installer follow the link.
- Supply skills from the directory the child already publishes.

## Decision Outcome

Chosen option: supply skills from the child's own directory — the writer the asset already has keeps the location it already documents, and the wrapper stops claiming a tree it does not write.

`<home>/.claude/skills` is linked into each session directory by the same declared link of [ADR-0103](./ADR-0103-permit-a-declared-link.md), and `skills` leaves the tree set. [The asset service](../../src/services/assets.rs) states the pairing once, so a launch, the catalog row, and the declared-links check read one list.

Asking installers to target the wrapper tree was rejected: it makes every project shipping a skill a function of this wrapper, which is the dependency ADR-0106 refused in the other direction. Keeping the anchor was rejected because a link is what a careful installer refuses, and that refusal is correct.

## Consequences

- Good: an installer writes an ordinary directory at the path the child documents.
- Good: one skill tree per machine, read by a wrapped and an unwrapped `claude` alike.
- Bad: `$HOME` becomes required and must be absolute, and the wrapper reads a path under it that it does not own.
- Bad: a container binds two paths where it bound one.

## Status

Implemented

Amends [ADR-0106](./ADR-0106-supply-child-assets-from-one-tree.md), which keeps its decision for the remaining nine. Enacted in [the asset service](../../src/services/assets.rs) and [the guide](../guides/supplying-child-assets.md). Shaped by [040](../plan/slices/040-native-skill-directory/README.md).
