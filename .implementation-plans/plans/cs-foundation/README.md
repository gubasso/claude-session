# claude-session — Foundation: Crate Skeleton, Plumbing & Quality Gates

> Complexity: L | Rounds: 4 | Generated: 2026-06-19 | Repo: /workspaces/claude-session

## Problem Statement

`claude-session` is a from-scratch Rust CLI that wraps the `claude` command (baseline v2.1.183 at `/home/gbasso/.local/bin/claude`). It enhances `claude` while NEVER breaking native passthrough: any unknown arg/flag/subcommand forwards verbatim (`OsString`-preserving) to the real `claude`, exactly as if the user ran stock `claude`. The repo today is a bare scaffold: a stub `src/main.rs` (`println!("Hello, world!")`), an `edition = "2024"` `Cargo.toml` with no dependencies, `rust-toolchain.toml` (`channel = "stable"`), and pre-commit tooling (`.pre-commit-config.yaml`, `committed.toml`, `.config/nextest.toml`, `.editorconfig`, `Cargo.lock`). There is no `src/` module tree, no `docs/`, no `CLAUDE.md`/`AGENTS.md`, no `justfile`, no `deny.toml`.

This directory builds the foundation every feature plan depends on: the canonical Rust cli-spec crate tree; the load-bearing plumbing (`error.rs` with `AppError::exit_code()`, `logging.rs`, single-writer `ui/`, immutable `AppContext`, figment `config/`); a clap passthrough skeleton with a _minimal_ working child spawn (so passthrough is demonstrable and testable now — the robust Spawner, signal forwarding, and recursion guard belong to `cs-wrapper-runtime`); and the Diátaxis `docs/`/ADR system plus the `justfile`/`deny.toml`/`CLAUDE.md`→`AGENTS.md` quality gates.

## Strategy

Bottom-up, foundations first. R1 lays the manifest, lints, blessed deps, and the canonical empty module tree so `cargo check` passes. R2 makes the cross-cutting plumbing real (error layers + the mandatory exit-code matrix test, tracing logging to XDG state, single-writer UI, `AppContext`, figment config). R3 builds the clap passthrough skeleton, wrapper-verb stubs, and a minimal inherited-env child spawn so `claude-session <native args>` actually runs `claude`. R4 establishes the docs/ADR system and quality gates. Each round leaves a compiling, test-covered base the next consumes.

## Rounds

1. `crate-manifest-and-module-tree.md` — Cargo.toml (blessed deps, strict lints, release profile, concrete rust-version), toolchain components, canonical empty module tree compiling.
2. `errors-logging-context-config.md` — thiserror layers + exit-code matrix test, tracing→XDG state, single-writer ui, immutable AppContext, figment Config.
3. `clap-passthrough-and-minimal-spawn.md` — root parser with verbatim external-subcommand passthrough, wrapper-verb stubs, argv normalization, minimal inherited-env child spawn, dispatch + main.rs.
4. `docs-adr-and-quality-gates.md` — Diátaxis docs/ skeleton, first ADRs, justfile→pre-commit, deny.toml, CLAUDE.md→AGENTS.md SoT.

## Execution Commands

```bash
# Execute the next todo round (executor reads queue-rounds.yaml, runs the first todo round, then stops):
/prex -ar @.implementation-plans/plans/cs-foundation/

# Or target a specific round file directly:
/prex -ar .implementation-plans/plans/cs-foundation/crate-manifest-and-module-tree.md
```

## Execution Discipline

**Rounds must be executed one at a time.** Each round is a self-contained unit of work designed for a single `/prex` session. Do not implement multiple rounds in one session.

When `/prex` is pointed at this directory or this `README.md`, it MUST:

1. Read this plan's `queue-rounds.yaml`.
2. Find the first round with status `todo`.
3. Set that round's `status` to `doing`, execute ONLY that round, then set it to `done` and stop.
4. End the session — a fresh `/prex` session is launched for any subsequent round.

## Decisions & Constraints

- `Executor: prex (EF 1.5)`.
- **Canonical crate tree is mandatory** (`rust/cli-spec/00-directory-tree.md`): single bin; `src/main.rs` ≤120 LOC (`parse → init logging → AppContext → dispatch → exit-code map`); `cli/` (clap derive only), `commands/` (one free `run(ctx, args) -> Result<(), AppError>` per verb), `domain/`, `services/`, `adapters/` (trait + impl), `config/` (figment), `context.rs`, `error.rs`, `logging.rs`, `ui/`, `util/`. Post-2018 module form (`foo.rs` + `foo/`, avoid `mod.rs`). Default visibility `pub(crate)`.
- **Start single-crate** (`01-crate-layout.md`); no workspace until explicit triggers (2nd binary, publishable subsystem, slow `cargo check`, ~8k LOC).
- **thiserror-per-layer + mandatory exit-code matrix test** (`03-error-handling.md`): `DomainError`, `<Sys>AdapterError`, `ServiceError`, `AppError`; `anyhow` only in `main`; `AppError::exit_code()` maps every variant explicitly to a BSD sysexits code — NO catch-all `_ => 1`.
- **Blessed deps only** (`07-dependencies.md`); avoid `dirs`/`chrono`/`lazy_static`/`serde_yaml`/ `env_logger`/`structopt`/`failure`. Commit `Cargo.lock`; enforce with `cargo deny`.
- **Dependencies are ALWAYS added with `cargo add`** (project-wide, every plan/round). A coding agent adds a dependency via `cargo add <crate> [--features ...]` — NEVER by hand-editing the `[dependencies]` table or hand-writing a version string — so cargo resolves the dependency graph, fetches the latest compatible version, and updates `Cargo.lock`. Deviate only with a **documented exception** (a known-broken latest, or a deliberately required exact pin), recorded in an ADR or a code/manifest comment.
- **Output discipline** (`cli-design/01-logging-and-output.md`): stdout = result only; stderr = everything else; all terminal output flows through the single `ui` writer; CI-linted via `rg` for stray `println!`/`eprintln!` outside `src/ui/` + `main.rs`.
- **Config precedence** `cli > env > project > user > defaults`; env prefix `CLAUDE_SESSION_` (nested `__`); `#[serde(deny_unknown_fields)]`; user config at `~/.config/claude-session/`.
- **pre-commit hooks are the SoT for quality gates**; `justfile` gate recipes delegate to `pre-commit run …`; inner-loop recipes (`build`/`run`/`fmt`/`watch`) stay raw cargo.
- **CLAUDE.md calls AGENTS.md; AGENTS.md is the lean SoT** (model: `/home/gbasso/Projects/_gubasso/cog/CLAUDE.md`).
- **Docs follow Diátaxis** (`docs-design/01-diataxis-zones.md`): `docs/decisions/`, `docs/guides/`, `docs/reference/`, `docs/explanation/`; lean ADRs ≤350 words, 5 sections, 5 statuses, never deleted.
- **Minimal spawn only here.** The robust `Spawner` trait, signal forwarding, exit-code mapping, and recursion guard live in `cs-wrapper-runtime`. Foundation ships just enough spawn to prove passthrough.

## Rejected Alternatives

- **Cargo workspace from day one** — rejected; cli-spec says start single-crate, migrate only on explicit triggers.
- **`anyhow` everywhere** — rejected; `anyhow` only in `main`; library boundaries return typed `thiserror` errors so `exit_code()` stays exhaustive.
- **Building the full Spawner/signal/recursion-guard here** — rejected; that is a coherent subsystem owned by `cs-wrapper-runtime`. Foundation keeps a minimal inherited-env spawn.
- **Importing code from the reference projects** — rejected (inspiration only, zero imports).

## Risks & Edge Cases

- `edition = "2024"` needs a recent stable toolchain; pin a concrete `rust-version` and verify `cargo check` before adding deps. (handled R1)
- The exit-code matrix is user-facing API; the mandatory test must enumerate every `AppError` variant so a future variant without a code fails the build. (handled R2)
- Output-discipline lint false positives on doc comments; scope `rg` to `src/`, exclude `src/ui/` + `main.rs`. (handled R4)

## Completion

When all rounds are done, set each round `done` in this plan's `queue-rounds.yaml` and set this plan `done` in the top-level `.implementation-plans/queue-plans.yaml`. Nothing moves on disk.
