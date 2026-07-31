# claude-session — Foundation: Crate Skeleton, Plumbing & Quality Gates

> Complexity: L | Rounds: 4 | Generated: 2026-06-19 | Repo: repository root

## Problem Statement

`claude-session` is a from-scratch Rust CLI that wraps the `claude` command, resolved from `PATH`. Which child versions it supports is a perishable fact owned by `docs/reference/prior-art.md` and `docs/decisions/ADR-0031-enforce-the-child-refresh-lock-version-floor.md`, not by this plan. It enhances `claude` while NEVER breaking native passthrough: any unknown arg/flag/subcommand forwards verbatim (`OsString`-preserving) to the real `claude`, exactly as if the user ran stock `claude`. The crate itself is still a stub — `src/main.rs` prints "Hello, world!" and `Cargo.toml` has an empty `[dependencies]` — but the surrounding project is fully bootstrapped: `rust-toolchain.toml`, `flake.nix`, `justfile`, `deny.toml`, `.config/nextest.toml`, `committed.toml`, `release-plz.toml`, install and publish scripts, CI workflows, dual licensing, `CLAUDE.md`, `AGENTS.md`, and a populated `docs/` tree carrying the specifications this plan implements against. There is still no `src/` module tree.

This directory builds the foundation every feature plan depends on: the canonical crate tree specified in `docs/explanation/architecture.md`; the load-bearing plumbing (`error.rs` with `AppError::exit_code()`, `logging.rs`, single-writer `ui/`, immutable `AppContext`, layered `config/`); and a passthrough skeleton with a _minimal_ working child spawn, so passthrough is demonstrable and testable now — the robust Spawner, signal forwarding, and recursion guard belong to `cs-wrapper-runtime`. The docs tree, the ADRs, the `justfile`, `deny.toml`, and the `CLAUDE.md`→`AGENTS.md` chain already exist; R4 closes out what remains rather than creating them.

## Strategy

Bottom-up, foundations first. R1 adds only the absent manifest lint, binary, and release-profile sections plus the canonical module tree; existing package metadata, the pinned toolchain, and the empty lockfile are already present. R2 makes the cross-cutting plumbing real (error layers + the mandatory exit-code matrix test, tracing logging to XDG state, single-writer UI, `AppContext`, layered config). R3 builds the argv pre-split and clap skeleton, wrapper-verb stubs, and a minimal inherited-env child spawn so `claude-session <native args>` actually runs `claude`. R4 closes out the remaining gate work and verifies the code against the specifications. Each round leaves a compiling, test-covered base the next consumes.

## Rounds

1. `crate-manifest-and-module-tree.md` — absent Cargo manifest lint/binary/release sections and the canonical documented module tree; no speculative dependency or repeated bootstrap work.
2. `errors-logging-context-config.md` — thiserror layers + exit-code matrix test, tracing→XDG state, single-writer ui, immutable AppContext, figment Config.
3. `clap-passthrough-and-minimal-spawn.md` — argv pre-split, root parser, wrapper-verb stubs, minimal inherited-env child spawn, dispatch + main.rs.
4. `docs-adr-and-quality-gates.md` — output-ownership lint hook, spec-versus-code conformance pass, queue closeout.

## Execution Commands

Any executor following [the contract](../../README.md#the-executor-contract) can run these rounds. `/prex` is the one used to generate them, shown here as a worked example:

```bash
# Execute the next todo round (executor reads queue-rounds.yaml, runs the first todo round, then stops):
/prex -ar @.implementation-plans/plans/cs-foundation/

# Or target a specific round file directly:
/prex -ar .implementation-plans/plans/cs-foundation/crate-manifest-and-module-tree.md
```

## Execution Discipline

Execution follows the executor contract in [`../../README.md`](../../README.md#the-executor-contract), which owns the rule: one round per session, first `todo` round only, status transitions in `queue-rounds.yaml`, stop.

This plan adds no exceptions to it.

## Decisions & Constraints

- **Executor provenance:** `prex (EF 1.5)` — the profile these rounds were generated with. Provenance only; see [the contract](../../README.md#the-executor-contract).
- **Canonical crate tree is mandatory** — exact module roles and prohibitions are owned by [`docs/explanation/architecture.md` § Module roles](../../../docs/explanation/architecture.md#module-roles), and every round inherits [`docs/reference/coding-conventions.md` § Implementation-round boundaries](../../../docs/reference/coding-conventions.md#implementation-round-boundaries).
- **The `xtask` trigger has fired** under [ADR-0014](../../../docs/decisions/ADR-0014-xtask-workspace-for-dev-tooling.md). The later configuration-generator round owns creating that tooling; foundation work must neither expose an unowned public API nor import tooling into the shipped target. See [`architecture.md` § One shipped crate, plus `xtask`](../../../docs/explanation/architecture.md#one-shipped-crate-plus-xtask).
- **thiserror-per-layer + mandatory exit-code matrix test** — the layer stack is specified in `docs/reference/coding-conventions.md` and the matrix in `docs/reference/exit-codes.md`: `DomainError`, `<Sys>AdapterError`, `ServiceError`, `AppError`; the boundary error type only in `main`; `AppError::exit_code()` maps every variant explicitly — NO catch-all `_ => 1`.
- **Dependency admission** is owned by [`docs/reference/dependencies.md` § Adding a dependency](../../../docs/reference/dependencies.md#adding-a-dependency). A crate enters the selected graph only when its round uses it; no round pre-adds a deferred crate or hand-writes a version.
- **Output discipline** — specified in `docs/reference/logging-and-output.md`: stdout = result only; stderr = everything else; all terminal output flows through the single `ui` writer; lint-enforced against stray `println!`/`eprintln!` outside `src/ui/` + `main.rs`.
- **Config precedence** `defaults < user < project < env < cli`, with the env prefix, nesting, and schema rules specified in `docs/reference/configuration.md`. Paths come from `docs/reference/xdg-storage.md`.
- **pre-commit hooks are the SoT for quality gates**; the gate map is in `docs/reference/testing-and-quality.md`. `justfile` gate recipes delegate to `pre-commit run …`; inner-loop recipes stay raw cargo.
- **`CLAUDE.md` calls `AGENTS.md`; `AGENTS.md` is the lean SoT.** Already in place — this round does not recreate it.
- **Docs are organized by reader need** — the zones, the ADR rules, and the placement policy are in `AGENTS.md` (Documentation Maintenance) and `docs/decisions/ADR-0012-docs-architecture.md`. The tree already exists; this round does not recreate it.
- **Minimal spawn only here.** The robust `Spawner` trait, signal forwarding, exit-code mapping, and recursion guard live in `cs-wrapper-runtime`. Foundation ships just enough spawn to prove passthrough.

## Rejected Alternatives

- **A broad library or tooling API** — rejected. ADR-0014 authorizes only the surface the `xtask` tooling needs; it does not turn the shipped crate into a general library.
- **A single opaque error type everywhere** — rejected; typed per-layer errors converge on a closed `AppError` so `exit_code()` stays exhaustive. See `docs/decisions/ADR-0008-layered-error-architecture.md`.
- **Building the full Spawner/signal/recursion-guard here** — rejected; that is a coherent subsystem owned by `cs-wrapper-runtime`. Foundation keeps a minimal inherited-env spawn.
- **Argv normalization before parsing** — rejected. Order, bytes, count, and empty arguments are all preserved; the pre-split consumes only wrapper-owned tokens and never rewrites what it forwards. See `docs/decisions/ADR-0002-verbatim-argv-passthrough.md`.

## Risks & Edge Cases

- The existing Rust 2024/MSRV metadata and pinned 1.97.1 toolchain must remain aligned; R1 verifies them rather than recreating them.
- The exit-code matrix is user-facing API; the mandatory test must enumerate every `AppError` variant so a future variant without a code fails the build. (handled R2)
- Output-discipline lint false positives on doc comments; scope the grep to `src/`, exclude `src/ui/` + `main.rs`. (handled R4)
- A derive parser alone cannot accept a leading unknown flag — that case is rejected before external-subcommand handling applies — so argv must be pre-split before parsing. See `docs/reference/cli-surface.md`. (handled R3)

## Completion

When all rounds are done, set each round `done` in this plan's `queue-rounds.yaml` and set this plan `done` in the top-level `.implementation-plans/queue-plans.yaml`. Nothing moves on disk.
