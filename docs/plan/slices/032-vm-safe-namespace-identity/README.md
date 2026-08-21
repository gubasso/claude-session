# 032 — VM-safe namespace identity

## Goal

Stop two kernels that share one state tree from sharing one session directory, which the namespace component alone cannot prevent because every kernel names its initial mount namespace with the same fixed inode.

## Appetite

1 implementation session.

## Core

Two runs that share the state tree but not their kernel receive different session directories.

## In scope

- One decision amending [ADR-0107](../../../decisions/ADR-0107-scope-a-terminal-to-its-namespace.md): the namespace component gains a per-kernel discriminator, so a virtual machine reaching a shared tree over a filesystem share never lands in the host's namespace directory.
- The machine identifier as the discriminator, because session directories are durable and must survive reboots; a fingerprint of it joins the namespace component rather than adding a path level.
- A fallback ladder for an unreadable or empty machine identifier: the boot identifier, honest at the cost of stranding per boot, and below that the rung names nothing and the existing refusal absorbs it.
- The `session-terminal-derives` report naming the discriminator beside the namespace, with no new check id ([ADR-0018](../../../decisions/ADR-0018-one-probe-set-with-stable-check-ids.md)).
- Alignment of [XDG storage](../../../reference/xdg-storage.md), [session isolation](../../../explanation/session-isolation.md), [accounts](../../../reference/accounts.md), and [doctor](../../../reference/doctor.md) where the component is spelled.

## Out of scope

- Migrating the session directories the rename strands, which [ADR-0102](../../../decisions/ADR-0102-key-child-state-by-terminal.md) already records as unpruned and a launch rebuilds.
- The peer registry's scope, which [031](../031-shared-peer-registry/README.md) keys by boot deliberately: registrations are boot-ephemeral where session directories are durable.
- Detecting virtualization, which is a heuristic answering a question the machine identifier answers exactly.
- The isolation stacks themselves; the wrapper reads what the kernel reports and never asks which product arranged it.

## Governed by

- [AGENTS.md](../../../../AGENTS.md)
- [Charter](../../charter.md)
- [ADR-0006](../../../decisions/ADR-0006-place-files-by-xdg-ownership.md)
- [ADR-0018](../../../decisions/ADR-0018-one-probe-set-with-stable-check-ids.md)
- [ADR-0046](../../../decisions/ADR-0046-support-linux-and-a-single-child-baseline.md)
- [ADR-0048](../../../decisions/ADR-0048-build-for-a-present-need.md)
- [ADR-0102](../../../decisions/ADR-0102-key-child-state-by-terminal.md)
- [ADR-0107](../../../decisions/ADR-0107-scope-a-terminal-to-its-namespace.md)
- [XDG storage](../../../reference/xdg-storage.md)
- [Doctor](../../../reference/doctor.md)
- [Coding conventions](../../../reference/coding-conventions.md)
- [Testing and quality](../../../reference/testing-and-quality.md)
- [Development workflow](../../../guides/development-workflow.md)

## Acceptance

- When two machine identifiers name one namespace inode, the derived components shall differ. -> domain::namespace::tests::two_machines_never_share_a_component
- Where the machine identifier is unreadable or empty, the boot identifier shall discriminate instead, and where neither yields a value, the rung shall name nothing rather than fall back to a shared name. -> domain::namespace::tests::an_empty_discriminator_names_nothing
- When this run's session identity is reported, the report shall name the discriminator beside the namespace. -> doctor::the_identity_report_names_the_discriminator
- When the work lands, no document shall describe the namespace component without its discriminator.

## Rabbit holes

- Keeping a guest's directories stable across its recreation, which nothing on the machine records; escape: a recreated guest is a new machine and a fresh directory is the honest result.
- Reading vendor identity files beyond the kernel's own two; escape: the ladder has exactly two rungs and a refusal.
- Migrating stranded directories, which cannot know which kernel wrote them; escape: they were already unpruned.

## Done when

Two kernels sharing one state tree never share a session directory, one kernel's directories survive its reboots, the report names the discriminator, and `just hooks` is green.

## Revisions

None.
