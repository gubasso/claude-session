# 031 — Shared peer registry

## Goal

Let every session on one host list and message the others through the child's own peer discovery, by sharing the one pid-keyed directory the per-terminal split needlessly divided.

## Appetite

1 implementation session.

## Core

Two sessions of one machine, in different terminals, projects, or accounts, appear in each other's peer listing.

## In scope

- One decision sharing the child's registry host-wide through a declared link, scoped by kernel boot and mount namespace ([ADR-0108](../../../decisions/ADR-0108-share-the-child-peer-registry-across-sessions.md)).
- A peer-scope derivation beside [ADR-0107](../../../decisions/ADR-0107-scope-a-terminal-to-its-namespace.md)'s namespace naming: one boot component read from the kernel's boot identifier, one mount-namespace component, each its own path element under `peers/`.
- The declared link at each session directory's `sessions` name, created by the same guard step that links `projects`, after adopting a real directory already at that name by moving its disposable entries into the registry.
- Degrading to an unshared launch, with a logged warning, when the scope cannot be derived or the registry is refused; awareness is additive and never blocks an exec.
- The registry directory and the link under the doctor's existing storage probes, with no new check id ([ADR-0018](../../../decisions/ADR-0018-one-probe-set-with-stable-check-ids.md)).
- Alignment of [XDG storage](../../../reference/xdg-storage.md) and [session isolation](../../../explanation/session-isolation.md), the latter gaining the container and virtual-machine sharing model with the mount declarations each stack needs.

## Out of scope

- Reading, parsing, or rendering any registration: the child owns the schema, and no [ADR-0089](../../../decisions/ADR-0089-carry-a-child-owned-fact-only-against-an-obligation.md) obligation covers a wrapper that consumes it.
- Pruning stale boot scopes, which carry only dead registrations and rebuild nothing; they are the namespace directories' posture again.
- The session-directory collision between separate kernels sharing one state tree, which predates this slice and is [032](../032-vm-safe-namespace-identity/README.md)'s to fix.
- Naming sessions, which the child already derives and `--name` already overrides through passthrough.
- Any new configuration key, wrapper flag, check id, or carried environment name.

## Governed by

- [AGENTS.md](../../../../AGENTS.md)
- [Charter](../../charter.md)
- [ADR-0006](../../../decisions/ADR-0006-place-files-by-xdg-ownership.md)
- [ADR-0018](../../../decisions/ADR-0018-one-probe-set-with-stable-check-ids.md)
- [ADR-0048](../../../decisions/ADR-0048-build-for-a-present-need.md)
- [ADR-0051](../../../decisions/ADR-0051-let-every-surface-element-discriminate.md)
- [ADR-0061](../../../decisions/ADR-0061-protect-storage-from-accidental-local-drift.md)
- [ADR-0089](../../../decisions/ADR-0089-carry-a-child-owned-fact-only-against-an-obligation.md)
- [ADR-0102](../../../decisions/ADR-0102-key-child-state-by-terminal.md)
- [ADR-0103](../../../decisions/ADR-0103-permit-a-declared-link.md)
- [ADR-0107](../../../decisions/ADR-0107-scope-a-terminal-to-its-namespace.md)
- [ADR-0108](../../../decisions/ADR-0108-share-the-child-peer-registry-across-sessions.md)
- [XDG storage](../../../reference/xdg-storage.md)
- [Coding conventions](../../../reference/coding-conventions.md)
- [Testing and quality](../../../reference/testing-and-quality.md)
- [Development workflow](../../../guides/development-workflow.md)

## Acceptance

- When two runs share one boot and one mount namespace, their session directories shall link one peer registry. -> session_storage::two_terminals_of_one_scope_link_one_registry
- When the boot or the mount namespace differs, the derived scopes shall differ. -> domain::peers::tests::a_different_boot_or_namespace_scopes_apart
- When a real directory occupies the linked name, a launch shall adopt its entries and replace it with the link. -> session_storage::an_existing_registry_directory_is_adopted
- Where no scope can be derived, the launch shall proceed with an unshared registry rather than refuse.

## Rabbit holes

- Making host and container sessions see each other, which their unshared sockets make a lie; escape: the scope key hiding them is the correct answer, not a gap.
- Verifying registrations or filtering stale ones, which is consuming the child's schema; escape: share the directory and stop.
- A doctor check that counts or ages peers; escape: the storage probes already say everything the wrapper knows.
- Locking the adoption move against a live child; escape: a lost race refuses the link once and the next launch retries clean.

## Done when

Two sessions of one machine list each other, existing terminals adopt their old registries on the next launch, an underivable scope still execs, and `just hooks` is green.

## Revisions

None.
