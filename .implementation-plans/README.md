# Implementation Plans

This directory stores implementation plans generated for staged agent execution.

`queue-plans.yaml` is the source of truth for plan status, order, dependencies, and execution prompts. Plans live under `plans/`; every plan is a directory containing round files and an inner `queue-rounds.yaml`.

Execute one round at a time. Status lives in YAML; files and directories do not move between states.

## Ownership boundary

Three things own three different questions, and keeping them apart is what stops this directory from becoming a second, drifting specification.

- **The queue YAML owns status.** What is queued, in progress, or done is read from and written to `queue-plans.yaml` and the per-plan `queue-rounds.yaml` — never inferred from the tree.
- **Round files own sequencing.** A round describes the work: its scope, what is deliberately excluded, its ordered steps, and its acceptance criteria.
- **The specifications under `docs/` own the contracts.** Exit codes, XDG placement, the CLI surface, the process runtime, configuration, coding conventions, and testing rules all live there, with the decisions behind them recorded as ADRs.

A round file therefore **cites** the specification that owns a rule rather than restating it, using a repo-relative path such as `docs/reference/exit-codes.md`. No round may cite a path outside this repository — see the self-containment rule in [AGENTS.md](../AGENTS.md).

When a round needs a rule that is not yet specified, add it to the owning specification and cite it; do not settle it inline. When a round and a specification disagree, the specification wins by default; the full reconciliation procedure is in [the development workflow](../docs/guides/development-workflow.md).
