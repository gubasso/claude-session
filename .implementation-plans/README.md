# Implementation Plans

This directory stores implementation plans generated for staged agent execution.

`queue-plans.yaml` is the source of truth for plan status, order, dependencies, and execution prompts. Plans live under `plans/`; every plan is a directory containing round files and an inner `queue-rounds.yaml`.

## Ownership boundary

Three things own three different questions, and keeping them apart is what stops this directory from becoming a second, drifting specification.

- **The queue YAML owns status.** What is queued, in progress, or done is read from and written to `queue-plans.yaml` and the per-plan `queue-rounds.yaml` — never inferred from the tree.
- **Round files own sequencing.** A round describes the work: its scope, what is deliberately excluded, its ordered steps, and its acceptance criteria.
- **The specifications under `docs/` own the contracts.** Exit codes, XDG placement, the CLI surface, the process runtime, configuration, coding conventions, and testing rules all live there, with the decisions behind them recorded as ADRs.

A round file therefore **cites** the specification that owns a rule rather than restating it, using a repo-relative path such as `docs/reference/exit-codes.md`. No round may cite a path outside this repository — see the self-containment rule in [AGENTS.md](../AGENTS.md).

When a round needs a rule that is not yet specified, add it to the owning specification and cite it; do not settle it inline. When a round and a specification disagree, the specification wins by default; the full reconciliation procedure is in [the development workflow](../docs/guides/development-workflow.md).

## The executor contract

An **executor** is whatever runs a round: an agent, a script, or a person working through the steps by hand. This section is the normative definition of what the queue expects from one. A named tool is only ever an _implementation_ of this contract, never the definition of it — which is what keeps this directory operable without any particular tool installed.

**Rounds are executed one at a time.** Each round file is a self-contained unit of work sized for a single session. Two rounds in one session is a contract violation, not a shortcut: status is the only coordination mechanism here, and it cannot represent partial progress spanning rounds.

An executor pointed at a **plan directory** or its `README.md` must:

1. Read that plan's `queue-rounds.yaml`.
2. Find the first round whose `status` is `todo`.
3. Set that round's `status` to `doing`, execute **only** that round, then set it to `done`.
4. Stop. Any subsequent round is a fresh session.

An executor pointed at a **single round file** executes that round and applies the same status transitions to it.

When every round in a plan is `done`, set the plan `done` in `queue-plans.yaml`. Status lives in YAML; files and directories never move between states.

### Provenance fields are not requirements

Plan and round headers carry two fields that record where the document came from. Neither places a requirement on the reader, and neither needs resolving to execute a round:

| Field                       | Means                               |
| --------------------------- | ----------------------------------- |
| `Generated: <date>`         | When the plan was written           |
| `Executor: <tool> (EF <n>)` | Which executor profile generated it |

The `EF` marker is **opaque provenance**. Nothing in this repository defines, resolves, or depends on it. A round is executable by any executor that follows the contract above.

Likewise, the `prompt:` field in a queue file is a **convenience string** for one particular tool — the load-bearing content is the path it points at, which any executor can act on. `/prex` appears in these files as one such tool; it is not a dependency of this repository, and nothing here requires it.
