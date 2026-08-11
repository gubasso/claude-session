# Development workflow

## Prepare

Enter the pinned development shell, install hooks once, and establish the baseline.

```bash
nix develop
just hooks-install
just hooks
```

Report a red baseline before adding unrelated repairs.

## Pick up current work

1. Open [milestones](../plan/milestones.md) and read `## in flight`.
2. Continue the `active` slice, or change the first `shaped` line to `active`. The section is ordered by execution, so that line is the next slice whose named predecessors are closed.
3. Open that line's slice `README.md`, at `slices/<id>-<slug>/`, and read it completely.
4. Load only the files named under `Governed by`; they are the slice's implementation filter.
5. Implement the core first, then `In scope` from top to bottom. If appetite binds, cut from the end.
6. Check off `tasks.md` only when it exists; it carries no requirements.
7. If work has begun and `Goal`, `Core`, `Appetite`, or `Acceptance` changes, add a dated entry under `Revisions` explaining what was learned.
8. Route a new blocker to [open questions](../plan/open-questions.md) instead of expanding another slice.
9. Run the slice's `Done when` checks and the full gate. Change the milestone status to `done` and move that line to `## closed` in the same change, then delete its `tasks.md`.

Use `cut` when the core shipped and the appetite ended the remainder; it is a success, and the note names what was dropped. Use `reshaped` when started work stopped uphill and was shaped again; the note names the successor slice id, which takes the next free id rather than the old one. Every terminal status moves the line to `## closed`, and a closed line never moves back. [Milestones](../plan/milestones.md) remains the only status surface, and a slice `README.md` never carries a status of its own.

## Add a wrapper command

Follow the [four-edit rule](../explanation/architecture.md#adding-a-verb-the-four-edit-rule): parser arguments, command enum, free command handler, then dispatch arm. Check the spelling against the [CLI surface](../reference/cli-surface.md); claiming a child spelling changes passthrough and requires a decision.

Send results to standard output and diagnostics to standard error through the single writer. Add verb-level `--json` only when the invocation produces scriptable data, then add the integration test and help snapshot.

Grammar changes reach the completions and the man page through the same parser tree, and nothing generated is checked in. Run `just artifacts` as tail work to regenerate into the ignored build directory and smoke the result.

```bash
just lint
just test-unit
just test-integration
just artifacts
```

## Add a dependency

1. Follow [dependency admission](../reference/dependencies.md#adding-a-dependency).
2. Add the crate and features through Cargo so the manifest and lockfile move together.
3. Use it in the same slice; the push gate rejects unused dependencies.
4. Record a hard-to-reverse dependency choice in an ADR.

```bash
cargo add <crate> --features <features>
just deny
just audit
```

## Write a test

Choose the lane from [testing and quality](../reference/testing-and-quality.md#what-each-lane-may-admit), make the fixture hermetic, and use the byte-recording child stub for process behavior. Extend the mandatory contract test instead of creating a parallel source of truth.

```bash
just test-unit
just test-integration
```

## Record or change a decision

Copy [the ADR template](../decisions/template.md), take the next stable id, list serious alternatives, and keep the whole file at or below 350 words. Follow [the lifecycle decision](../decisions/ADR-0078-adopt-the-seven-state-decision-lifecycle.md) for authority and preserve every record from `Proposed` onward through supersession, deprecation, rejection, or amendment.

When a live owner and another document disagree, change the non-owner. If the owner itself must change, update it and the significant decision in one change. Put unresolved blockers in [open questions](../plan/open-questions.md).

## Work with drafts

Use `.draft/` only as ignored scratch space. Rewrite durable substance into its owner and remove the draft so it cannot compete with shipped documentation.

## Before proposing a change

```bash
just hooks
```

This runs both hook stages. Fix the reported cause; do not bypass or substitute a partial gate. Branch and release rules live in the [release workflow](../reference/release-workflow.md#branch-and-release-invariant).
