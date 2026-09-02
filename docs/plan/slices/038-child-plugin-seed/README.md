# 038 — Child plugin seed

## Goal

Make the plugins a user declares reach every session directory, so an isolated configuration directory stops costing them the language servers, and every other plugin capability, that a durable directory would have kept.

## Appetite

2 implementation sessions.

## Core

A launch supplies the child's plugin state from one read-only tree the user maintains once per machine, and the child loads those plugins on the first run of a session directory that never existed before.

## In scope

- One decision supplying plugins from a read-only seed, against the launch obligation of [ADR-0089](../../../decisions/ADR-0089-carry-a-child-owned-fact-only-against-an-obligation.md), amending the plugin exclusion [ADR-0106](../../../decisions/ADR-0106-supply-child-assets-from-one-tree.md) recorded when no present need reached it.
- A seed location under the wrapper's own data base per [ADR-0006](../../../decisions/ADR-0006-place-files-by-xdg-ownership.md), beside the asset tree rather than inside it, because the asset set is the child's published user-scope names and this is not one of them.
- The child's own seed variable, set at launch, carried against the same obligation as the two the wrapper already sets.
- A launch-time copy of the plugin state the seed holds into the session directory, in the shape [ADR-0105](../../../decisions/ADR-0105-seed-a-session-at-launch.md) already uses for the questions a directory's newness makes the child ask.
- One configuration key retiring the child's plugin recommendation, which an always-fresh directory asks again in every session.
- One catalog check under [ADR-0018](../../../decisions/ADR-0018-one-probe-set-with-stable-check-ids.md), reporting what a launch would supply.
- A guide covering the one-time population step, since the wrapper writes no user configuration ([ADR-0015](../../../decisions/ADR-0015-retire-the-init-verb.md)).
- Alignment of [XDG storage](../../../reference/xdg-storage.md), [process runtime](../../../reference/process-runtime.md), and [configuration](../../../reference/configuration.md).

## Out of scope

- Linking the child's plugin tree into the session directory, which the measurement in the record rejects.
- Adding the child's plugin flags to the argument vector, which [ADR-0002](../../../decisions/ADR-0002-verbatim-argv-passthrough.md) reserves and no obligation here requires.
- Building the seed for the user, or validating what it contains; the wrapper supplies a directory and copies opaque bytes ([ADR-0088](../../../decisions/ADR-0088-model-nothing-the-child-already-owns.md)).
- Sharing one mutable plugin tree between sessions, which is the collision [ADR-0102](../../../decisions/ADR-0102-key-child-state-by-terminal.md) split the directory to end.
- Reconciling a session's plugin writes back into the seed, which would make two writers of one tree.

## Governed by

- [AGENTS.md](../../../../AGENTS.md)
- [Charter](../../charter.md)
- [ADR-0002](../../../decisions/ADR-0002-verbatim-argv-passthrough.md)
- [ADR-0006](../../../decisions/ADR-0006-place-files-by-xdg-ownership.md)
- [ADR-0015](../../../decisions/ADR-0015-retire-the-init-verb.md)
- [ADR-0018](../../../decisions/ADR-0018-one-probe-set-with-stable-check-ids.md)
- [ADR-0048](../../../decisions/ADR-0048-build-for-a-present-need.md)
- [ADR-0051](../../../decisions/ADR-0051-let-every-surface-element-discriminate.md)
- [ADR-0057](../../../decisions/ADR-0057-build-the-child-environment-by-prefix-scrub-and-marker.md)
- [ADR-0088](../../../decisions/ADR-0088-model-nothing-the-child-already-owns.md)
- [ADR-0089](../../../decisions/ADR-0089-carry-a-child-owned-fact-only-against-an-obligation.md)
- [ADR-0105](../../../decisions/ADR-0105-seed-a-session-at-launch.md)
- [ADR-0106](../../../decisions/ADR-0106-supply-child-assets-from-one-tree.md)
- [XDG storage](../../../reference/xdg-storage.md)
- [Process runtime](../../../reference/process-runtime.md)
- [Configuration](../../../reference/configuration.md)
- [Doctor](../../../reference/doctor.md)
- [Child facts](../../../reference/child-facts.yaml)
- [Coding conventions](../../../reference/coding-conventions.md)
- [Testing and quality](../../../reference/testing-and-quality.md)
- [Development workflow](../../../guides/development-workflow.md)

## Acceptance

- When a launch materialises a session directory and the seed tree exists, it shall point the child at that tree and copy the plugin state the tree holds into the session directory.
- Where the seed tree does not exist, the launch shall set no seed variable and write no plugin state.
- Where an ambient seed variable is set, the launch shall drop it whether or not it supplies one of its own, so a session reaches the tree the wrapper selected rather than one it inherited.
- When the wrapper copies plugin state, it shall never write into the seed tree.
- When the recommendation key is enabled, a launch shall record it in the session directory beside the two answers already written there.
- When the work lands, the argument vector shall still carry exactly the composed settings pair ahead of the user's untouched suffix.

## Rabbit holes

- Modelling the child's plugin schema to merge, validate, or repair a seed; escape: the wrapper copies two files as bytes and reads neither.
- Making the seed writable so a session's installs persist; escape: two writers of one tree is the drift this project rejects everywhere else, and the child documents the seed as read-only.
- Growing a verb that builds or refreshes a seed; escape: the guide names the child's own commands, the operator runs them.
- Treating the recommendation key as a settings concern; escape: it lives in the file a directory's newness makes the child ask from, which is where [ADR-0105](../../../decisions/ADR-0105-seed-a-session-at-launch.md) already writes.
- Reaching for the child's plugin flags when the ordering surprises; escape: the ordering is met by seeding the directory, which is a wrapper concern, and the flags are the child's.

## Done when

A first launch into a fresh session directory loads the plugins the seed declares, an absent seed changes nothing, the seed is unwritten afterwards, the guide names the one-time step, and `just hooks` is green.

## Revisions

None.
