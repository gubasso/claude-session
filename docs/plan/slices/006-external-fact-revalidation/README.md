# 006 — External fact revalidation

## Goal

Revalidate the externally owned facts this repository depends on, and record what a blocked measurement leaves unproven.

## Appetite

1 implementation session.

## Core

Every entry taken up is checked against its recorded procedure in full, and its date carries whatever the procedure could not prove.

## In scope

- Execute the recorded revalidation procedure for each entry taken up.
- Update owning pages for measured drift.
- Attempt the concurrent-refresh measurement the entries name, and record its outcome either way.

## Out of scope

- A proxy integration guide, which cannot be followed without carrying a child-owned name the wrapper never sets ([ADR-0089](../../../decisions/ADR-0089-carry-a-child-owned-fact-only-against-an-obligation.md)).
- Extending prior art, which no present boundary asks for once the guide is cut.
- Compression, rewriting, routing, or a wrapper-owned environment injection surface.

## Governed by

- [AGENTS.md](../../../../AGENTS.md)
- [ADR-0089](../../../decisions/ADR-0089-carry-a-child-owned-fact-only-against-an-obligation.md)
- [Wrapper model](../../../explanation/wrapper-model.md)
- [Process runtime](../../../reference/process-runtime.md)
- [XDG storage](../../../reference/xdg-storage.md)
- [Research tracking](../../../reference/research-tracking.yaml)
- [Development workflow](../../../guides/development-workflow.md)

## Acceptance

- When an entry is taken up, the maintainer shall execute its recorded procedure in full and update its owner for measured drift.
- If a step depends on an open question, then the entry shall record the blocking measurement rather than an unqualified date.
- Where the wrapper neither sets, reads, nor implements a child-owned mechanism, the repository shall carry no guide for it.

## Rabbit holes

- Implementing the proxy itself; escape: the seam is inheritance and the wrapper does nothing.
- Forcing a refresh window that fires on the child's schedule; escape: record the blocking measurement and move on.
- Revalidating entries that are not due; escape: take up the ones nearest their cadence and leave the rest.

## Done when

Each entry taken up records a current check date or a named blocking measurement, its owner page agrees with what was measured, and `just hooks` is green.

## Revisions

- 2026-08-12: The proxy guide and the prior-art extension are cut, and the slice is renamed from `proxy-guide-and-research`. A guide a user can follow has to write the child's base-URL variable into this repository, and the wrapper never sets, reads, or implements it; a guide without that name restates one sentence [process runtime](../../../reference/process-runtime.md) already carries. The prior-art rows existed only to arm a future refusal of a wrapper-side injection surface, which [ADR-0089](../../../decisions/ADR-0089-carry-a-child-owned-fact-only-against-an-obligation.md) now settles directly. What remains is the revalidation, which is what the entries were tracked for.
