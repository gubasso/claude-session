# 009 — Release readiness

<!-- markdownlint-configure-file { "MD043": { "headings": ["*","## Goal","## Appetite","## Core","## In scope","## Out of scope","## Governed by","## Acceptance","## Rabbit holes","## Done when","## Revisions"] } } -->

## Goal

Prove the complete product is ready for its first release.

## Appetite

1 implementation session.

## Core

The cross-subsystem launch passes, implemented ADRs close accurately, and the full gate is green.

## In scope

- User README and install notes.
- Dependency and security audit.
- External release-bootstrap verification.

## Out of scope

- New product features.
- Claiming external operator prerequisites without measurement.

## Governed by

- [AGENTS.md](../../../../AGENTS.md)
- [README](../../../../README.md)
- [Releasing guide](../../../guides/releasing.md)
- [Release workflow](../../../reference/release-workflow.md)
- [Testing and quality](../../../reference/testing-and-quality.md)
- [ADR-0022](../../../decisions/ADR-0022-cut-the-first-release-when-passthrough-works.md)

## Acceptance

- When the complete stubbed launch runs, the wrapper shall preserve passthrough while applying the selected account and composed profile.
- When ADR implementation evidence exists, the record shall move to `Implemented` with an in-repository link.
- When release gates run, the repository shall pass the complete pre-commit and pre-push verdict without bypasses.
- If any external bootstrap prerequisite is unverified, then the slice shall remain not done under Q-005.

## Rabbit holes

- Feature polish beyond release correctness; escape: cut from the end of `In scope`.
- Performing forge-side operator actions; escape: measure and record them without claiming local control.

## Done when

The cross-subsystem integration checks, dependency and security gates, full `just hooks` verdict, and Q-005 measurement all pass.

## Revisions

None.
