# 040 — Native skill directory

## Goal

Let a skill installer write the directory `claude` publishes, as an ordinary directory, and still have every session reach what it wrote. Today the asset tree is the anchor, so that path is a symbolic link, and an installer that refuses to write through one fails.

## Appetite

1 implementation session.

## Core

A launch supplies skills from the child's own configuration directory and the remaining nine assets from the wrapper's tree, with one list stating which name comes from where.

## In scope

- One decision supplying skills from the child's own directory, amending the single-tree rule of [ADR-0106](../../../decisions/ADR-0106-supply-child-assets-from-one-tree.md) for that one name.
- The child's own configuration directory name, carried against the launch obligation of [ADR-0089](../../../decisions/ADR-0089-carry-a-child-owned-fact-only-against-an-obligation.md), resolved from `$HOME` beside the four XDG bases.
- One declared pairing of asset name to source directory, read by the launch, the catalog row, and the declared-links check alike, so the three cannot disagree.
- `$HOME` required, and required to be absolute, so the skill source can never resolve to a relative path that a session link would then store verbatim.
- Alignment of the [assets guide](../../../guides/supplying-child-assets.md), [doctor](../../../reference/doctor.md), and [XDG storage](../../../reference/xdg-storage.md).

## Out of scope

- Writing, creating, or correcting the child's skill directory; the wrapper reads it for presence and nothing else ([ADR-0088](../../../decisions/ADR-0088-model-nothing-the-child-already-owns.md)).
- Moving the other nine assets out of the wrapper tree, which have no writer but the user and no reason to move.
- A second catalog check for the new source; one row answers what a session gets, and its message names the directory behind each half ([ADR-0051](../../../decisions/ADR-0051-let-every-surface-element-discriminate.md)).
- Migrating an existing anchor, or removing a link some other program left at the native path; that is the operator's one-time step and the guide names it.
- Relaxing the storage guard's symbolic-link refusal, which is what makes a declared link the only link the wrapper accepts ([ADR-0103](../../../decisions/ADR-0103-permit-a-declared-link.md)).

## Governed by

- [AGENTS.md](../../../../AGENTS.md)
- [Charter](../../charter.md)
- [ADR-0006](../../../decisions/ADR-0006-place-files-by-xdg-ownership.md)
- [ADR-0015](../../../decisions/ADR-0015-retire-the-init-verb.md)
- [ADR-0018](../../../decisions/ADR-0018-one-probe-set-with-stable-check-ids.md)
- [ADR-0048](../../../decisions/ADR-0048-build-for-a-present-need.md)
- [ADR-0051](../../../decisions/ADR-0051-let-every-surface-element-discriminate.md)
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

- When a launch materialises a session directory and the child's skill directory exists, it shall seat `skills` as a declared link to that directory rather than to the wrapper's tree.
- Where the child's skill directory does not exist, the launch shall create nothing in its place and shall still supply every asset the tree holds. -> accounts::a_launch_reads_skills_from_the_child_directory_and_never_the_tree
- Where `$HOME` is unset, empty, or relative, every verb shall fail at path resolution, whether or not all four XDG bases resolve on their own. -> domain::paths::tests::an_unusable_home_is_refused_even_when_every_base_resolves
- When `base-dirs-resolve` reports a resolution failure, its remedy shall name `$HOME` first, so a reader with four absolute bases is not sent to the two variables that already resolve. -> doctor::an_unusable_home_is_reported_with_a_remedy_that_names_it
- When the declared-links check runs, it shall probe the skill seat against the child's directory, so a repointed skill link fails the report as it fails the next launch.
- When both sources hold assets, the catalog row shall name the directory behind each half.
- When neither source holds an asset, the catalog row shall warn rather than fail.

## Rabbit holes

- Validating, creating, or repairing the child's skill directory because the wrapper now names it; escape: it is read for presence, exactly as the tree is.
- Generalising the source into a per-asset search path; escape: one name has a second source, and a list of two answers it.
- Serving a run that has four absolute XDG bases and no usable `$HOME`; escape: nobody has one, and serving it would mean an optional source, an optional seat target, and a third seat state.
- Adding a catalog row per source; escape: the reader's question is what a session gets, which one row answers.
- Teaching the storage guard about paths outside the managed region; escape: the guard validates the seat, and a declared target has never been validated.
- Reading back the variable the wrapper sets to relocate the child's configuration directory; escape: it names the session directory the wrapper just chose, so the native one comes from `$HOME` instead.

## Done when

A launch seats `skills` at the child's own directory, an installer writes that directory with no link in the way, an absent directory changes nothing else, an unusable `$HOME` is refused at resolution, the guide names the one-time move, and `just hooks` is green.

## Revisions

None.
