# Foundation R4: Docs/ADR System & Quality Gates

> Plan: cs-foundation | Round: 4 of 4 | Complexity: L | Executor: prex (EF 1.5) | Generated: 2026-06-19 | Repo: /workspaces/claude-session

## Context

`claude-session`'s rules, principles, and docs must be self-contained in the repo, following the docs-design reference (Diátaxis zones, lean ADRs, ARID single-SoT). `CLAUDE.md` must call `AGENTS.md`, and `AGENTS.md` must be the lean SoT. Quality gates: pre-commit hooks are the source of truth; a `justfile` delegates to them. Rounds 1–3 produced the compiling crate, the plumbing, and the clap passthrough skeleton with a minimal spawn. This round documents the foundation and locks in the gates so feature plans build on a documented, enforced base.

## Previous Rounds

Round 1: crate tree + manifest. Round 2: error/logging/context/config plumbing. Round 3: clap passthrough skeleton, wrapper-verb stubs, minimal spawn, dispatch + main. Expect a working wrapper that passes native args through and stubs its own verbs.

## Scope of This Round

- IN scope: `docs/` Diátaxis skeleton (`docs/README.md` index, `docs/decisions/`, `docs/guides/`, `docs/reference/`, `docs/explanation/`); a gitignored `.draft/`; the first ADRs (lean template, ≤350 words, 5 sections, 5 statuses) recording the decided architecture — non-breaking passthrough contract; thiserror-per-layer + sysexits exit-code policy; pty-keyed multiplexer-agnostic isolation approach; JSON-pieces + YAML-manifest config-composition model; managed-`claude login` seed→session auth model; status `Accepted` for these binding decisions; `justfile` (gate recipes delegate to `pre-commit run …`; inner-loop recipes raw cargo); `deny.toml` (license allowlist, ban wildcards/unknown sources); the stdout/stderr-ownership `rg` lint as a pre-commit hook + `just` recipe; `CLAUDE.md` (calls `@AGENTS.md` + a few non-negotiables); `AGENTS.md` (lean SoT: scope, module conventions, four-edit subcommand rule, error/logging/output rules, quality-gate principle, docs ownership).
- OUT of scope: feature ADRs requiring implemented behavior; the prior-art competitive-analysis note (deferred to `cs-docs-hardening` where the research is finalized).

## Current State

### Key Files

- `/workspaces/claude-session/.pre-commit-config.yaml` — already wired (7.5KB); reuse/extend its hooks.
- `/workspaces/claude-session/committed.toml` — Conventional Commits, 72-col.
- `/workspaces/claude-session/.config/nextest.toml` — nextest profiles already present.
- No `docs/`, `CLAUDE.md`, `AGENTS.md`, `justfile`, or `deny.toml` yet.

### Existing Patterns

CLAUDE.md→AGENTS.md model: `/home/gbasso/Projects/_gubasso/cog/CLAUDE.md` is essentially `@AGENTS.md` plus a few "Non-negotiable:" lines; `AGENTS.md` is the SoT. codex-session's `CLAUDE.md` states "pre-commit hooks are the Source of Truth for quality gates" and its `justfile` gate recipes call `pre-commit run …` while inner-loop recipes (`build`/`run`/`fmt`/`watch`) stay raw cargo. Lean ADR template at `/home/gbasso/Projects/docs-n-notes/tech/programming/docs-design/template-adr.md`: five sections (Context and Problem Statement, Considered Options, Decision Outcome, Consequences, Status), ≤350 words, filename `ADR-<number>-<decision>.md`, one Status of {Proposed, Accepted, Implemented, Superseded, Rejected}. Diátaxis zones: zone-first, topic-second; topic dirs live inside a zone; root `docs/README.md` is an index only.

## Implementation Steps

### First Step: Mark this round as started

In this plan's `queue-rounds.yaml`, set this round's (`item: docs-adr-and-quality-gates`) `status` to `doing`.

### Step 1: Diátaxis docs skeleton

Create `docs/README.md` (index only) and the four zone dirs (`docs/decisions/`, `docs/guides/`, `docs/reference/`, `docs/explanation/`) each with a brief index/`.gitkeep`. Add a gitignored `.draft/`.

### Step 2: First ADRs

Write `ADR-0001…` (lean template) for the decided architecture (passthrough contract; exit-code policy; isolation approach; config-composition model; auth model). Status `Accepted`.

### Step 3: Quality gates

Write `deny.toml` (license allowlist incl. `MIT`/`Apache-2.0`, ban wildcard versions + unknown sources). Write `justfile`: gate recipes (`test`, `lint`, `audit`, `deny`, `check`) delegate to `pre-commit run <hook> --all-files`; inner-loop recipes (`build`, `run`, `fmt`, `fix`, `watch`, `clean`, `install`) raw cargo. Add the stdout/stderr-ownership `rg` lint hook + recipe. Ensure `cargo deny`/`fmt`/`clippy`/`nextest` hooks exist in `.pre-commit-config.yaml`.

### Step 4: CLAUDE.md → AGENTS.md

Write `CLAUDE.md` as `@AGENTS.md` plus a short "Non-negotiable:" list (passthrough contract, exit-code matrix, output discipline, quality-gate SoT). Write `AGENTS.md` as the lean SoT — including the **dependency rule: every dependency is added with `cargo add`, never by hand-editing `[dependencies]` or writing version strings** (cargo resolves the latest compatible version and updates `Cargo.lock`).

### Final Step: Update the queue

1. In this plan's `queue-rounds.yaml`, set this round's (`item: docs-adr-and-quality-gates`) `status` to `done`.
2. All rounds are now done, so in the top-level `.implementation-plans/queue-plans.yaml` set this plan's (`item: cs-foundation`) `status` to `done`. Leave the plan directory in place.

## Acceptance Criteria

- [ ] `docs/` has the four Diátaxis zones and an index-only `docs/README.md`; topic dirs (if any) are inside a zone.
- [ ] At least five lean ADRs exist (each ≤350 words, 5 sections, one valid Status).
- [ ] `CLAUDE.md` calls `@AGENTS.md`; `AGENTS.md` is the lean SoT.
- [ ] `just lint` / `just test` delegate to `pre-commit run …`; `deny.toml` exists and `cargo deny check` passes.
- [ ] `pre-commit run --all-files` passes.
- [ ] This plan's `queue-rounds.yaml` shows round `docs-adr-and-quality-gates` as `done` and the top-level `queue-plans.yaml` shows `cs-foundation` as `done`.

## Next Round

This is the final round of this plan. Downstream sibling plans (`cs-isolation`, then `cs-wrapper-runtime`, `cs-accounts-auth`, `cs-config-composition`, `cs-docs-hardening`) build the features on this foundation, wired by `depends_on`.
