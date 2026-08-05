# Docs & Hardening R4: ADR Closeout & Release Hardening

> Plan: cs-docs-hardening | Round: 4 of 4 | Complexity: M | Executor: prex (EF 1.5) | Generated: 2026-06-19 | Repo: repository root

## Context

This final round closes out the project: it transitions the foundational ADRs from `Accepted` to `Implemented` (preserving history — never deleting superseded ADRs), and hardens the project for release — supply-chain checks clean, a user-facing `README.md`, install notes, and a final cross-subsystem integration pass. After this round, the full `cs-` plan series is complete. All prior plans (foundation, isolation, wrapper-runtime, accounts-auth, config-composition) and this plan's rounds 1–3 are done.

## Previous Rounds

All `cs-` feature plans + this plan R1 (docs), R2 (doctor), R3 (completions/man). Expect the full feature set, docs, and quality gates to exist.

## Scope of This Round

- IN scope: transition the ADRs in `docs/decisions/` whose decisions are now implemented from `Accepted` to `Implemented` (do NOT delete any ADR; add new ADRs for any decision discovered during implementation); a user-facing top-level `README.md` (what claude-session is, quick start, the account/profile/isolation model, headroom integration pointer, paths table); install notes (cargo install / justfile install); a final release-hardening pass — `cargo deny check` + `cargo
  audit` clean, `Cargo.lock` committed, `pre-commit run --all-files` green; a final cross-subsystem integration test pass (passthrough + isolation + a managed account + a composed profile, against a stubbed `claude`).
- OUT of scope: new features; behavior changes beyond hardening.

## Current State

### Key Files

- `docs/decisions/` — ADRs to transition to `Implemented`.
- `README.md` — update the existing file for shipped behaviour.
- `deny.toml`, `.pre-commit-config.yaml` — supply-chain gates.
- `tests/` — final cross-subsystem integration test.

### Existing Patterns

The status vocabulary, what each value means, and which values are current authority are specified in `docs/reference/project-governance.md` § Decision status. Read that table rather than the summary in `AGENTS.md`; a record is **never deleted**, a changed decision is superseded with a forward link, and a partly-changed one keeps its status and gains an `Amended by ADR-NNNN` line. Transitioning a record also has to satisfy the exact-value sweep in `docs/reference/testing-and-quality.md` § Documentation sweeps: the status value stands alone on the first nonblank line, with any evidence in the paragraph below it.

The release decisions this round closes against are `docs/decisions/ADR-0020-adopt-a-two-branch-release-model.md`, `docs/decisions/ADR-0022-cut-the-first-release-when-passthrough-works.md`, `docs/decisions/ADR-0066-ship-one-linux-artifact.md`, and `docs/decisions/ADR-0073-defer-openssf-scorecard-until-the-first-release.md`.

`README.md` already exists and carries the install, development-shell, task, licence, and contribution sections, plus a design-contract summary explicitly marked as a summary of the normative sources. This round **updates** it for shipped behaviour rather than writing it: replace the pre-implementation warning, add the quick start and the account/profile/isolation model, and link the docs index. Do not duplicate the paths table or the exit-code matrix — link `docs/reference/xdg-storage.md` and `docs/reference/exit-codes.md`, or the two copies will drift.

Release gates are already wired; see `docs/reference/testing-and-quality.md`. Confirm they are clean rather than re-adding them. The gate is `just hooks`, which runs both hook stages; `pre-commit run --all-files` selects files rather than stages and silently omits the push half.

This round's release work depends on operator actions the repository cannot perform or observe: a published `develop`, the installed GitHub App with its two secrets, and the rulesets created in the order `docs/reference/release-workflow.md` § Forge enforcement requires. They are steps 2, 4, and 5 of `docs/guides/releasing.md` § Bootstrap release automation once. Verify they are done before treating a release as reachable.

## Implementation Steps

### First Step: Mark this round as started

In this plan's `queue-rounds.yaml`, set this round's (`item: adr-closeout-and-release`) `status` to `doing`.

### Step 1: ADR closeout

Transition implemented ADRs to `Implemented` (preserve history); add ADRs for any decisions made during implementation.

### Step 2: User README + install notes

Write the top-level `README.md` (intro, quick start, account/profile/isolation model, headroom pointer, paths table, exit-code contract) and install notes.

### Step 3: Release hardening

Ensure `cargo deny check` + `cargo audit` clean, `Cargo.lock` committed, `just hooks` green on both stages.

### Step 4: Final integration pass

Add/confirm a cross-subsystem integration test (passthrough + isolation + a managed account + a composed profile) against a stubbed `claude`.

### Final Step: Update the queue

1. In this plan's `queue-rounds.yaml`, set this round's (`item: adr-closeout-and-release`) `status` to `done`.
2. All rounds are now done, so in the top-level `.implementation-plans/queue-plans.yaml` set this plan's (`item: cs-docs-hardening`) `status` to `done`. Leave the plan directory in place.

## Acceptance Criteria

- [ ] Implemented ADRs are transitioned to `Implemented`; no ADR is deleted; new decisions get ADRs.
- [ ] A user-facing `README.md` exists (intro, quick start, model, headroom pointer, paths, exit codes).
- [ ] `cargo deny check` + `cargo audit` are clean; `Cargo.lock` committed; `just hooks` is green on both stages.
- [ ] A cross-subsystem integration test passes against a stubbed `claude`.
- [ ] This plan's `queue-rounds.yaml` shows round `adr-closeout-and-release` as `done` and the top-level `queue-plans.yaml` shows `cs-docs-hardening` as `done`.

## Next Round

This is the final round of the final plan. The `cs-` series is complete; `claude-session` is feature- complete and release-hardened.
