# 029 — Machine-global child assets

## Goal

Make one machine-local set of user-authored child assets reach every session directory, so an isolated configuration directory stops costing the user the skills, agents, and rules they wrote once.

## Appetite

2 implementation sessions.

## Core

A session directory the wrapper materialises carries the user's own assets, from one place they maintain once per machine.

## In scope

- One decision materialising a declared set of user-authored child assets into every session directory, against the launch obligation of [ADR-0089](../../../decisions/ADR-0089-carry-a-child-owned-fact-only-against-an-obligation.md): an isolated configuration directory cannot reach them by any other route the child offers.
- An asset location under the wrapper's own data base, per [ADR-0006](../../../decisions/ADR-0006-place-files-by-xdg-ownership.md), rather than the predecessor's tree, which the self-containment rule in [AGENTS.md](../../../../AGENTS.md) forbids depending on.
- The asset set itself, taken from the child's published user-scope tree, with an absent member skipped rather than materialised empty.
- One catalog check under [ADR-0018](../../../decisions/ADR-0018-one-probe-set-with-stable-check-ids.md), reporting which assets a launch would supply.
- A guide covering the one-time population step, since the wrapper writes no user configuration ([ADR-0015](../../../decisions/ADR-0015-retire-the-init-verb.md)).
- Alignment of [XDG storage](../../../reference/xdg-storage.md) for the new base and its artifacts.

## Out of scope

- The child's plugin tree, which no present need reaches ([ADR-0048](../../../decisions/ADR-0048-build-for-a-present-need.md)) and whose sanctioned mechanism is a separate read-only seed rather than a link.
- Copying or synchronising asset content, which would drift from the one tree the user maintains.
- Modelling what any asset contains; the wrapper supplies a directory and the child reads it ([ADR-0088](../../../decisions/ADR-0088-model-nothing-the-child-already-owns.md)).
- Migrating the predecessor's tree automatically, which is the operator's move and belongs in the guide.
- The link mechanism itself, which 028 builds and this slice only uses.

## Governed by

- [AGENTS.md](../../../../AGENTS.md)
- [Charter](../../charter.md)
- [ADR-0006](../../../decisions/ADR-0006-place-files-by-xdg-ownership.md)
- [ADR-0015](../../../decisions/ADR-0015-retire-the-init-verb.md)
- [ADR-0018](../../../decisions/ADR-0018-one-probe-set-with-stable-check-ids.md)
- [ADR-0048](../../../decisions/ADR-0048-build-for-a-present-need.md)
- [ADR-0061](../../../decisions/ADR-0061-protect-storage-from-accidental-local-drift.md)
- [ADR-0088](../../../decisions/ADR-0088-model-nothing-the-child-already-owns.md)
- [ADR-0089](../../../decisions/ADR-0089-carry-a-child-owned-fact-only-against-an-obligation.md)
- [ADR-0103](../../../decisions/ADR-0103-permit-a-declared-link.md)
- [ADR-0106](../../../decisions/ADR-0106-supply-child-assets-from-one-tree.md)
- [XDG storage](../../../reference/xdg-storage.md)
- [Doctor](../../../reference/doctor.md)
- [Child facts](../../../reference/child-facts.yaml)
- [Coding conventions](../../../reference/coding-conventions.md)
- [Testing and quality](../../../reference/testing-and-quality.md)
- [Development workflow](../../../guides/development-workflow.md)

## Acceptance

- When a launch materialises a session directory, it shall supply every asset the user's asset tree holds. -> accounts::a_launch_supplies_the_assets_the_tree_holds_and_no_others
- Where the asset tree holds no member of the declared set, the launch shall supply nothing at that name rather than an empty one.
- When an asset is supplied, the child shall discover it exactly as it discovers one in its own configuration directory.
- Where an asset name already exists in the session directory as something the wrapper did not declare, the wrapper shall refuse. -> accounts::an_undeclared_occupant_at_an_asset_name_is_refused
- When the work lands, no document shall name the predecessor's tree as a location the wrapper reads at run time.

## Rabbit holes

- Growing the asset set to every name the child has ever read; escape: the set is the child's published user-scope tree, and a name outside it needs its own reason.
- Building plugin support because the tree sits beside the assets; escape: the plugin route is a read-only seed with its own record, and no present need reaches it.
- Writing a migration verb for a one-time copy; escape: the guide names the command, the operator runs it once.
- Making the set configurable to avoid deciding it; escape: a default nobody can name is not a default, and a typo becoming a missing skill is worse than a list.

## Done when

A session directory carries the user's own assets from the wrapper's asset tree, an absent asset is simply absent, the guide names the one-time step, and `just hooks` is green.

## Revisions

None.
