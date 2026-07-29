# Docs & Hardening R1: Revalidate Prior Art & Proxy-Integration Guide

> Plan: cs-docs-hardening | Round: 1 of 4 | Complexity: L | Executor: prex (EF 1.5) | Generated: 2026-06-19 | Repo: repository root

## Context

Prior art has already been surveyed and written up: `docs/reference/prior-art.md` classifies the account-switcher landscape, wrapper and shim design, Rust CLI structure, configuration layering, process supervision, and sandboxing, and records the perishable native-child facts the design rests on. Those facts expire, and the survey has a date. This round **revalidates and extends** that page rather than writing a competing one, and adds the guide that shows the proxy seam in use. The seam itself (environment injection into the child) was built in `cs-wrapper-runtime`; this round documents how to use it.

## Previous Rounds

The docs tree, `docs/reference/prior-art.md`, and `docs/reference/research-tracking.yaml` already exist — they predate this plan and are not this round's to create. `cs-wrapper-runtime` built the child-environment injection seam. Expect all of these to exist.

## Scope of This Round

- IN scope: **revalidating and extending the existing** `docs/reference/prior-art.md` — walk every entry in `docs/reference/research-tracking.yaml` whose cadence is due, re-verify it, update `last_checked`, and record what changed; then extend the page with the product landscape the engineering survey did not cover (further account-switching and session-wrapping tools, config-layering and sandboxing inspirations) and with any project that has since solved a problem this one still has open. Each addition keeps the page's existing shape: public URL, area inspected, pattern, what was borrowed or why it was rejected, access date, and facts distinguished from inference. Plus a proxy-integration guide in `docs/guides/` showing a concrete fronting proxy end to end — the exact commands, the environment key injected, and the "no compression, rewriting, or routing inside the wrapper" boundary.
- OUT of scope: **creating a second prior-art document.** One page, one home; a competing note in another zone would drift. Also out: restating the seam's contract in the guide (it belongs to `docs/explanation/wrapper-model.md` and `docs/reference/process-runtime.md` — the guide exercises it, it does not redefine it); doctor hardening (round 2); completions and man pages (round 3); ADR closeout and release (round 4).

## Current State

### Key Files

- `docs/reference/prior-art.md` — extend and revalidate; do not create a sibling.
- `docs/reference/research-tracking.yaml` — drives what to re-check; update `last_checked`.
- `docs/guides/` — already exists (it holds the development workflow); add the proxy-integration guide here.
- `docs/README.md` — index; add the new guide, and only the new guide.

### Existing Patterns

**`docs/reference/prior-art.md` already exists** and carries the engineering prior art — the account and session switchers, wrapper and shim design, Rust CLI structure, configuration layering, process supervision, and sandboxing — each entry naming what was inspected and what was borrowed or rejected. It also records the perishable native-child facts. This round **extends and revalidates that one page**; it does not create a second prior-art document, because two would drift.

Revalidation is driven by `docs/reference/research-tracking.yaml`: re-check each tracked fact, update `last_checked`, and record what changed. Extension means the product landscape the engineering survey did not cover, and any project that has since solved a problem this one still has open.

The proxy seam is documented as a **general environment-injection mechanism** in `docs/explanation/wrapper-model.md` and `docs/reference/process-runtime.md`. The guide this round adds shows a concrete fronting proxy end to end; it must not restate the seam's contract, only exercise it — and it must state the boundary that the wrapper implements no compression, rewriting, or routing.

Zone placement follows `AGENTS.md` (Documentation Maintenance): the guide goes in `docs/guides/`, which already exists and holds the development workflow.

## Implementation Steps

### First Step: Mark this round as started

In this plan's `queue-rounds.yaml`, set this round's (`item: prior-art-and-headroom-docs`) `status` to `doing`.

### Step 1: Revalidate the tracked facts

Work through `docs/reference/research-tracking.yaml`. For each entry whose cadence is due, follow its `revalidate` instruction, update `last_checked`, and correct the owning document if the fact changed. Pay particular attention to the native-child entries — the configuration-directory variable, the credential file layout, the trust-state file, the settings schema, and the version floor — since the whole isolation design rests on them and none carries a stability guarantee.

### Step 2: Extend the prior art

Extend `docs/reference/prior-art.md` with the product landscape and anything new since the last survey. Follow the page's existing column shape and its rules: only projects actually inspected, public URLs only, facts marked apart from inference, and no entry that says merely that something is popular.

### Step 3: Proxy-integration guide

Write a guide in `docs/guides/` showing a concrete fronting proxy end to end: start it, point the child at it through the wrapper's environment-injection seam, verify it is in the path. Give exact commands. State the boundary — the wrapper composes environment keys and implements no compression, rewriting, or routing — and **link** the seam's contract rather than restating it.

### Step 4: Link + lint

Add the new guide to the `docs/README.md` index. `docs/reference/prior-art.md` is already indexed, so do not add a second entry. Run `pre-commit run --all-files`.

### Final Step: Update the queue

1. In this plan's `queue-rounds.yaml`, set this round's (`item: prior-art-and-headroom-docs`) `status` to `done`.

## Acceptance Criteria

- [ ] Every due entry in `docs/reference/research-tracking.yaml` has been revalidated, with `last_checked` updated and any owning document corrected.
- [ ] `docs/reference/prior-art.md` is extended in its existing shape; **no second prior-art document was created**.
- [ ] A proxy-integration guide gives exact commands, states the no-compression boundary, and links the seam's contract rather than restating it.
- [ ] The new guide is linked from `docs/README.md`; `pre-commit run --all-files` passes.
- [ ] This plan's `queue-rounds.yaml` shows round `prior-art-and-headroom-docs` as `done`.

## Next Round

Round 2 (`doctor-hardening`) turns `doctor` into a comprehensive, graceful health check across all subsystems.
