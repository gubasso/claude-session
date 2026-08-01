# claude-session — Integration, Docs & Hardening

> Complexity: L | Rounds: 4 | Generated: 2026-06-19 | Repo: repository root

## Problem Statement

With the foundation, isolation, runtime, accounts/auth, and config-composition in place, `claude-session` needs its integration documented and its surface hardened for release. This plan documents the headroom/proxy integration (`ANTHROPIC_BASE_URL`, which the child env inherits), hardens `doctor` into a full graceful health check, finalizes shell completions + version + man pages, captures the web prior-art/competitive-analysis research into the repo docs (a brief requirement), and closes out the ADRs and release hardening. Depends on `cs-wrapper-runtime`, `cs-accounts-auth`, and `cs-config-composition` (it documents and hardens their behavior).

## Strategy

Four rounds. R1 writes the prior-art/research docs + the headroom integration guide into the Diátaxis zones. R2 hardens `doctor` into a comprehensive non-aborting health check across all subsystems. R3 finalizes `clap_complete` completions, the `version` verb, and `clap_mangen` man pages. R4 closes out ADRs (Accepted → Implemented), and does release hardening (cargo deny/audit clean, README user docs, install notes, final integration pass).

## Rounds

1. `prior-art-and-headroom-docs.md` — prior-art/competitive-analysis note + headroom integration guide.
2. `doctor-hardening.md` — comprehensive, graceful `doctor` across config/child/session/accounts/auth.
3. `completions-and-manpages.md` — `clap_complete` completions, `version`, `clap_mangen` man pages.
4. `adr-closeout-and-release.md` — ADR status closeout + release hardening + final integration pass.

## Execution Commands

Any executor following [the contract](../../README.md#the-executor-contract) can run these rounds. `/prex` is the one used to generate them, shown here as a worked example:

```bash
# Execute the next todo round (executor reads queue-rounds.yaml, runs the first todo round, then stops):
/prex -ar @.implementation-plans/plans/cs-docs-hardening/

# Or target a specific round file directly:
/prex -ar .implementation-plans/plans/cs-docs-hardening/prior-art-and-headroom-docs.md
```

## Execution Discipline

Execution follows the executor contract in [`../../README.md`](../../README.md#the-executor-contract), which owns the rule: one round per session, first `todo` round only, status transitions in `queue-rounds.yaml`, stop.

This plan adds no exceptions to it.

## Decisions & Constraints

- **Executor provenance:** `prex (EF 1.5)` — the profile these rounds were generated with. Provenance only; see [the contract](../../README.md#the-executor-contract).
- **Docs are organized by reader need**, per `AGENTS.md` (Documentation Maintenance) and `docs/decisions/ADR-0012-docs-architecture.md`: prior art is reference and **already exists** at `docs/reference/prior-art.md`; the proxy integration walkthrough is a guide; decisions are lean ADRs within the word budget, five sections, never deleted.
- **Revalidate and extend the existing prior art** rather than writing a second page. `docs/reference/prior-art.md` already classifies the switcher landscape, wrapper and shim design, configuration layering, process supervision, and sandboxing, and records the perishable native-child facts. `docs/reference/research-tracking.yaml` drives what to re-check and when.
- **headroom is integration-by-seam, not internal**: document exporting `ANTHROPIC_BASE_URL` → `headroom proxy` as the supported pattern; claude-session implements no compression and no injection surface.
- **doctor degrades, never aborts**: every catalog check runs independently, a failure never aborts the rest, and each one carries the four-part error shape from `docs/reference/exit-codes.md`. The catalog, the report, the grammar, and the exit rule are all specified in `docs/reference/logging-and-output.md`.
- **Release hardening**: `cargo deny`/`cargo audit` clean; user-facing `README.md`; install notes; pin/commit `Cargo.lock`.

## Rejected Alternatives

- **Implementing token compression internally** — rejected; only the env/wrap seam (headroom is the external tool).
- **Skipping the research capture** — rejected; the brief explicitly requires saving + classifying the prior-art findings into the repo docs.
- **A doctor that aborts on the first failure** — rejected; it must report all subsystem health gracefully.

## Risks & Edge Cases

- The native `claude` config/session surface and `CLAUDE_CONFIG_DIR` behavior are undocumented and can change; the docs must note this and the `doctor` checks must be defensive.
- Completions/man pages must reflect the final flag set (run after the feature plans).
- ADR closeout must not delete superseded ADRs (history is preserved).

## Completion

When all rounds are done, set each round `done` in this plan's `queue-rounds.yaml` and set this plan `done` in the top-level `.implementation-plans/queue-plans.yaml`. Nothing moves on disk. This is the final plan in the `cs-` series.
