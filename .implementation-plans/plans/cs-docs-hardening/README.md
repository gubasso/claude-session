# claude-session — Integration, Docs & Hardening

> Complexity: L | Rounds: 4 | Generated: 2026-06-19 | Repo: /workspaces/claude-session

## Problem Statement

With the foundation, isolation, runtime, accounts/auth, and config-composition in place,
`claude-session` needs its integration documented and its surface hardened for release. This plan
documents the headroom/proxy integration (the `ANTHROPIC_BASE_URL` seam built in `cs-wrapper-runtime`),
hardens `doctor` into a full graceful health check, finalizes shell completions + version + man pages,
captures the web prior-art/competitive-analysis research into the repo docs (a brief requirement), and
closes out the ADRs and release hardening. Depends on `cs-wrapper-runtime`, `cs-accounts-auth`, and
`cs-config-composition` (it documents and hardens their behavior).

## Strategy

Four rounds. R1 writes the prior-art/research docs + the headroom integration guide into the Diátaxis
zones. R2 hardens `doctor` into a comprehensive non-aborting health check across all subsystems. R3
finalizes `clap_complete` completions, the `version` verb, and `clap_mangen` man pages. R4 closes out
ADRs (Accepted → Implemented), and does release hardening (cargo deny/audit clean, README user docs,
install notes, final integration pass).

## Rounds

1. `prior-art-and-headroom-docs.md` — prior-art/competitive-analysis note + headroom integration guide.
2. `doctor-hardening.md` — comprehensive, graceful `doctor` across config/child/session/accounts/auth.
3. `completions-and-manpages.md` — `clap_complete` completions, `version`, `clap_mangen` man pages.
4. `adr-closeout-and-release.md` — ADR status closeout + release hardening + final integration pass.

## Execution Commands

```bash
# Execute the next todo round (executor reads queue-rounds.yaml, runs the first todo round, then stops):
/prex -ar @.implementation-plans/plans/cs-docs-hardening/

# Or target a specific round file directly:
/prex -ar .implementation-plans/plans/cs-docs-hardening/prior-art-and-headroom-docs.md
```

## Execution Discipline

**Rounds must be executed one at a time.** Each round is a self-contained unit of work designed for a
single `/prex` session. Do not implement multiple rounds in one session.

When `/prex` is pointed at this directory or this `README.md`, it MUST:

1. Read this plan's `queue-rounds.yaml`.
2. Find the first round with status `todo`.
3. Set that round's `status` to `doing`, execute ONLY that round, then set it to `done` and stop.
4. End the session — a fresh `/prex` session is launched for any subsequent round.

## Decisions & Constraints

- `Executor: prex (EF 1.5)`.
- **Docs follow Diátaxis** (`docs-design`): the prior-art/competitive-analysis note → `docs/explanation/`
  (or `docs/reference/`); the headroom integration → `docs/guides/`; decisions → `docs/decisions/`
  (lean ADRs, ≤350 words, 5 sections, 5 statuses, never deleted).
- **Capture the web research** (brief §10 + §9): the classified prior-art landscape (aisw, the
  cc-account-switchers, claude-swap, kustomize, bubblewrap/firejail, native CLAUDE_CONFIG_DIR facts)
  and the headroom integration contract must be saved into the repo docs and cited as INSPIRATION.
- **headroom is integration-by-seam, not internal**: document `ANTHROPIC_BASE_URL` → `headroom proxy`
  as the supported pattern; claude-session implements no compression.
- **doctor degrades, never aborts** (shell-tool lesson): report each subsystem's health with a
  three/four-part error + hint; never abort the whole check on one bad subsystem.
- **Release hardening**: `cargo deny`/`cargo audit` clean; user-facing `README.md`; install notes;
  pin/commit `Cargo.lock`.

## Rejected Alternatives

- **Implementing token compression internally** — rejected; only the env/wrap seam (headroom is the
  external tool).
- **Skipping the research capture** — rejected; the brief explicitly requires saving + classifying the
  prior-art findings into the repo docs.
- **A doctor that aborts on the first failure** — rejected; it must report all subsystem health
  gracefully.

## Risks & Edge Cases

- The native `claude` config/session surface and `CLAUDE_CONFIG_DIR` behavior are undocumented and can
  change; the docs must note this and the `doctor` checks must be defensive.
- Completions/man pages must reflect the final flag set (run after the feature plans).
- ADR closeout must not delete superseded ADRs (history is preserved).

## Completion

When all rounds are done, set each round `done` in this plan's `queue-rounds.yaml` and set this plan
`done` in the top-level `.implementation-plans/queue-plans.yaml`. Nothing moves on disk. This is the
final plan in the `cs-` series.
