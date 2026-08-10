# 009 — Release readiness

## Goal

Make the already-earned passthrough product ready for a separate human decision to release `0.1.0`.

## Appetite

1 implementation session.

## Core

First-release readiness proves ADR-0022 and the one-time bootstrap prerequisites without waiting for later features or performing an irreversible publish.

## In scope

- User README and installation notes limited to working passthrough behavior.
- Dependency, security, package-content, and complete repository gates for the `0.1.0` candidate.
- Measurement and recording of Q-005's external bootstrap prerequisites.
- Verification that the releasing guide keeps the one-time `publish-new` path and routine per-rung releases distinct.

## Out of scope

- Running `cargo publish`, creating a tag, cutting a release, or making the human release decision.
- Account, profile, doctor, completion, or man-page feature claims.
- Reopening this one-time slice for later versions; each later feature rung owns its documentation and release gates.

## Governed by

- [AGENTS.md](../../../../AGENTS.md)
- [README](../../../../README.md)
- [Releasing guide](../../../guides/releasing.md)
- [Release workflow](../../../reference/release-workflow.md)
- [Testing and quality](../../../reference/testing-and-quality.md)
- [ADR-0022](../../../decisions/ADR-0022-cut-the-first-release-when-passthrough-works.md)

## Acceptance

- When the `0.1.0` candidate is described, the repository shall claim only the passthrough behavior ADR-0022 requires.
- When package and release gates run, the repository shall pass the complete documented verdict without bypasses.
- When Q-005 is measured, its owning documents shall record the observed external prerequisites without presenting unverified forge state as live.
- When the one-time release path is reviewed, the releasing guide shall retain the manual `publish-new` boundary, Trusted Publishing handoff, and token revocation order.
- When a later feature rung becomes release-ready, its owning slice shall carry that version's documentation and gates without reopening slice 009.

## Rabbit holes

- Publishing while proving readiness; escape: stop at dry runs and leave the one-way decision to the human operator.
- Feature polish beyond passthrough correctness; escape: ADR-0022 owns the first-release threshold.
- Claiming external operator prerequisites without measurement; escape: keep Q-005 open until the forge state is observed.

## Done when

The README and install notes are honest for passthrough, package, dependency, security, and full `just hooks` gates pass, Q-005 is resolved, and no publishing command has run.

## Revisions

None.
