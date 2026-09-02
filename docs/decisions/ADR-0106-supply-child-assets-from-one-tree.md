# ADR-0106: Supply child assets from one wrapper-owned tree

## Context and Problem Statement

An isolated configuration directory has none of the skills, agents, or rules the user wrote: the variable relocating it relocates every path the child reads. There is no per-asset search path, and the only other route is the plugin system, which namespaces every skill it carries. Under [ADR-0102](./ADR-0102-key-child-state-by-terminal.md) such a directory now appears per terminal, so the gap is met in every pane.

## Considered Options

- Read them from the predecessor's tree.
- Package them as a plugin, enabled through the composed settings.
- Copy them into each session directory.
- Link them from one wrapper-owned tree.

## Decision Outcome

Chosen option: link them from one tree, at `<data>/assets/`, using the declared link of [ADR-0103](./ADR-0103-permit-a-declared-link.md). The user maintains it once per machine; every session directory reaches it.

The set is the child's published user-scope tree, so membership is a fact about the child rather than a wrapper preference. A member the tree does not hold is not linked: an empty directory claims the user has none, and they may not have written any yet.

The predecessor's tree was rejected: depending on it makes this wrapper a function of a program it does not ship, which self-containment forbids. It is the migration source, named in a guide, never read at run time.

The plugin route was rejected on ergonomics: plugin skills are always namespaced, so every skill the user invokes by hand gains a prefix. Copying was rejected because two trees drift.

`plugins/` is excluded: nothing needs it, and a linked one reports as corrupted.

## Consequences

- Good: one tree, reaching every terminal of every account.
- Good: the wrapper models no asset's content; it supplies a directory.
- Bad: the tree must be populated once, and nothing does it for the user.
- Bad: ten more child-owned names are carried, none reachable by the discovery scan.

## Status

Implemented

Rests on [ADR-0103](./ADR-0103-permit-a-declared-link.md). Enacted in [the asset service](../../src/services/assets.rs) and [the guide](../guides/supplying-child-assets.md). Shaped by [029](../plan/slices/029-machine-global-child-assets/README.md).

Amended by [ADR-0117](./ADR-0117-supply-plugins-from-a-read-only-seed.md); `plugins/` is still never linked.
