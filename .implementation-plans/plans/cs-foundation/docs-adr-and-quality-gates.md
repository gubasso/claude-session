# Foundation R4: Conformance & Gate Closeout

> Plan: cs-foundation | Round: 4 of 4 | Complexity: M | Executor: prex (EF 1.5) | Generated: 2026-06-19 | Repo: repository root

## Context

This round was originally scoped to create the docs tree, the first ADRs, `CLAUDE.md`, `AGENTS.md`, the `justfile`, and `deny.toml`. **All of that already exists.** A dedicated documentation build-out landed the specifications, twelve ADRs, and the governance chain, and the project bootstrap landed the task runner and supply-chain policy. Recreating any of it would produce a second, conflicting source of truth — the exact drift the documentation model exists to prevent.

What remains is the part that genuinely depends on rounds 1–3 having produced code: the output-ownership lint that could not be written before there was a `src/` tree, and a conformance pass verifying that the code those rounds produced actually matches the specifications it was written against. Rounds 1–3 produced the compiling crate, the plumbing, and the passthrough skeleton with a minimal spawn.

## Previous Rounds

Round 1: crate tree + manifest. Round 2: error/logging/context/config plumbing. Round 3: argv pre-split, clap skeleton, wrapper-verb stubs, minimal spawn, dispatch + main. Expect a working wrapper that passes native args through and stubs its own verbs.

## Scope of This Round

- IN scope: the stdout/stderr-ownership lint as a pre-commit hook plus a `just` recipe; the dependency-direction boundary lint (`domain/` imports nothing from `adapters/` or `services/`); a **conformance pass** comparing the code produced by rounds 1–3 against `docs/explanation/architecture.md`, `docs/reference/coding-conventions.md`, `docs/reference/exit-codes.md`, `docs/reference/logging-and-output.md`, `docs/reference/configuration.md`, and `docs/reference/cli-surface.md`, correcting whichever side is wrong; confirming the mandatory tests for this stage exist per `docs/reference/testing-and-quality.md` (exit-code matrix, golden argv, `--` sentinel, help snapshot).
- OUT of scope: creating any docs zone, ADR, `CLAUDE.md`, `AGENTS.md`, `justfile`, or `deny.toml` — all already exist. Transitioning ADR statuses from `Accepted` to `Implemented` (owned by `cs-docs-hardening` R4). Feature behaviour of any kind.

## Current State

### Key Files

- `.pre-commit-config.yaml` — fully wired; extend it, do not rewrite it.
- `justfile` — gate recipes already delegate to `pre-commit run …`; add the lint recipe alongside them.
- `deny.toml` — present with a filled allow-list.
- `docs/` — populated: an index, four zones, twelve ADRs. The specifications this round checks the code against.
- `AGENTS.md`, `CLAUDE.md` — present. `CLAUDE.md` is a one-line `@AGENTS.md` import and stays that way.
- `.config/nextest.toml` — profiles present; see the `--profile` foot-gun in `docs/reference/testing-and-quality.md`.

### Existing Patterns

Quality-gate ownership is settled and documented in `docs/reference/testing-and-quality.md`: pre-commit hooks are the source of truth, `justfile` gate recipes delegate to `pre-commit run …`, and inner-loop recipes (`build`, `run`, `fmt`, `watch`) stay raw cargo. The boundary lints are defined in the same page. The ADR rules — five sections, a word budget with a margin, one status from a closed vocabulary, never deleted — are in `AGENTS.md` under Documentation Maintenance, with the template at `docs/decisions/template.md`.

## Implementation Steps

### First Step: Mark this round as started

In this plan's `queue-rounds.yaml`, set this round's (`item: docs-adr-and-quality-gates`) `status` to `doing`.

### Step 1: Output-ownership lint

Add a pre-commit hook that fails on a print macro in `src/` outside `src/ui/` and `src/main.rs`, plus a matching `just` recipe. Scope the grep to `src/`, and make sure it does not fire on an example inside a `///` doc comment — that false positive is why this lint waited for real code.

### Step 2: Dependency-direction lint

Add a hook asserting that `src/domain/` imports nothing from `adapters/` or `services/`. The type system does not enforce this; without a lint it erodes silently.

### Step 3: Conformance pass

Read the specifications listed in scope and compare them against what rounds 1–3 actually built. Check specifically: the module tree and its prohibitions; `main.rs` at or under 120 lines doing the five steps; the exit-code matrix implemented exactly, with no catch-all arm; the stream contract and single output writer; the config precedence direction (`defaults < user < project < env < cli`); the argv pre-split preserving order, bytes, count, and **empty arguments**.

Where code and specification disagree: **the specification wins by default** and the code is corrected. If the code is actually right, update the specification _and_ the ADR carrying that decision. If it is a genuinely open question, add a new ADR with status `Proposed`. Do not leave two live claims in the repository.

### Step 4: Mandatory tests present

Confirm the tests that lock down this stage's contracts exist and pass: the exit-code matrix (exhaustive, no catch-all), the golden-argv table, the `--` sentinel, and the help snapshot. Add any that are missing.

### Final Step: Update the queue

1. In this plan's `queue-rounds.yaml`, set this round's (`item: docs-adr-and-quality-gates`) `status` to `done`.
2. All rounds are now done, so in the top-level `.implementation-plans/queue-plans.yaml` set this plan's (`item: cs-foundation`) `status` to `done`. Leave the plan directory in place.

## Acceptance Criteria

- [ ] A print macro outside `src/ui/` and `src/main.rs` fails the hook; a doc-comment example does not.
- [ ] An import from `adapters/` or `services/` into `domain/` fails the hook.
- [ ] Every discrepancy found in the conformance pass is resolved, with the resolution recorded — code corrected, or specification plus ADR updated, or a `Proposed` ADR added.
- [ ] The exit-code matrix test is exhaustive with no catch-all; adding a variant without a code fails the build.
- [ ] The golden-argv table passes, including the empty-argument and non-UTF-8 cases.
- [ ] No docs zone, ADR, `CLAUDE.md`, `AGENTS.md`, `justfile`, or `deny.toml` was created or rewritten by this round.
- [ ] `pre-commit run --all-files` passes.
- [ ] This plan's `queue-rounds.yaml` shows round `docs-adr-and-quality-gates` as `done` and the top-level `queue-plans.yaml` shows `cs-foundation` as `done`.

## Next Round

This is the final round of this plan. Downstream sibling plans (`cs-isolation`, then `cs-wrapper-runtime`, `cs-accounts-auth`, `cs-config-composition`, `cs-docs-hardening`) build the features on this foundation, wired by `depends_on`.
