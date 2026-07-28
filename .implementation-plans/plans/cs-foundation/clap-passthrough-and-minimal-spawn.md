# Foundation R3: Clap Passthrough Skeleton & Minimal Spawn

> Plan: cs-foundation | Round: 3 of 4 | Complexity: L | Executor: prex (EF 1.5) | Generated: 2026-06-19 | Repo: /workspaces/claude-session

## Context

`claude-session` MUST never break native `claude` passthrough: any unknown arg/flag/subcommand forwards verbatim (`OsString`-preserving) to the real `claude`, and a bare `claude-session ...` runs `claude ...`. claude-session-owned global flags must never collide with native `claude` flags. This round builds the clap skeleton that routes wrapper-verbs vs. passthrough, the wrapper-verb stubs, and a **minimal** inherited-env child spawn so passthrough is demonstrable and testable now. The robust `Spawner` trait, full signal forwarding, exit-code mapping, recursion guard, and isolated child-env injection are deliberately deferred to the `cs-wrapper-runtime` plan. Rounds 1–2 produced the crate tree and the error/logging/context/config plumbing.

## Previous Rounds

Round 1: canonical module tree + manifest. Round 2: `AppError` + exit-code matrix, tracing logging, single-writer `Ui`, immutable `AppContext`, figment `Config`. Expect `error.rs`/`logging.rs`/`ui/`/ `context.rs`/`config/` to be real and tested.

## Scope of This Round

- IN scope: `cli/mod.rs` root `Cli` parser (`allow_external_subcommands = true`, `subcommand_negates_reqs = true`, `disable_version_flag` so `-V` is wrapper-owned) with a `GlobalArgs` flatten (wrapper-owned, distinctively named: `-v/--verbose` count, `-q/--quiet`, `--format text|json`, `--config <path>`, `--dry-run`, plus reserved STUB flags `--account`, `--session`/`--group`, `--profile` for later plans) and a `Commands` enum whose `External(Vec<OsString>)` variant (`#[command(external_subcommand)]`) forwards everything else; `cli/argv.rs` argv normalization; intrinsic wrapper-verb STUBS (`version`, `completion`, `config`, `doctor`, `init` — each a free `pub fn run(ctx, args) -> Result<(), AppError>` printing "not yet implemented" via `Ui` except `version`, which prints our version + resolved child path/version); a minimal `commands/pass_through.rs` that spawns the real `claude` with **inherited env** (no isolation) via `std::process::Command`, waits, and propagates the child exit code; `commands/dispatch.rs`; `main.rs` (`parse → init logging → AppContext → dispatch → exit-code map`, ≤120 LOC).
- OUT of scope: the hexagonal `Spawner` trait, signal forwarding, recursion guard, isolated `CLAUDE_CONFIG_DIR` injection (all in `cs-wrapper-runtime`); session dirs (`cs-isolation`); accounts (`cs-accounts-auth`); config composition (`cs-config-composition`).

## Current State

### Key Files

- `/workspaces/claude-session/src/cli.rs` (+ `src/cli/`) — placeholder; becomes the root parser.
- `/workspaces/claude-session/src/commands.rs` (+ `src/commands/`) — placeholder; becomes dispatch + pass_through + verb handlers.
- `/workspaces/claude-session/src/main.rs` — minimal; becomes the real entrypoint.

### Existing Patterns

Reference root-parser shape (codex-session, inspiration only): `#[command(name = "...", allow_external_subcommands = true, subcommand_negates_reqs = true)]` with `#[command(flatten)] global: GlobalArgs` and `command: Option<Commands>`; the `Commands` enum ends in `#[command(external_subcommand)] External(Vec<OsString>)` forwarding all non-clap args. argv forwarded **verbatim preserving `OsString`** (non-UTF-8 safe). Wrapper-design rules (`cli-design/06-cli-wrapper-design`): keep the wrapper grammar small and explicit; default to verbatim pass-through, denylist the flags you claim; argv layout `mywrap [WRAPPER-OPTS] <verb> [--]
[CHILD-ARGS...]`; `--` is the end-of-options sentinel. Intrinsic top-level verbs: `version` (our version + resolved child path+version), `help`, `completion`, `config`, `doctor`, `init`.

### Process model note

claude-session needs post-exit work (credential sync-back, trust sync) in later plans, so it **spawn-and-waits** rather than `exec`s. This round's minimal spawn does the simplest correct thing: inherit env, run `claude`, propagate exit code. `cs-wrapper-runtime` replaces it with the robust path.

## Implementation Steps

### First Step: Mark this round as started

In this plan's `queue-rounds.yaml`, set this round's (`item: clap-passthrough-and-minimal-spawn`) `status` to `doing`.

### Step 1: Root parser + GlobalArgs + Commands enum

Build `cli/mod.rs` with the `Cli` struct, `GlobalArgs` (wrapper-owned, `global = true`), and the `Commands` enum ending in `External(Vec<OsString>)`. Claim `-V` via `disable_version_flag`. Reserve (but stub) `--account`, `--session`/`--group`, `--profile`.

### Step 2: argv normalization

Add `cli/argv.rs` to pre-parse/normalize argv (strip stray empties, support `--`) before clap parse, preserving `OsString`.

### Step 3: Minimal pass-through spawn

In `commands/pass_through.rs`, resolve `claude` via a simple `which`/PATH lookup (full resolution chain arrives in `cs-wrapper-runtime`), spawn with inherited env via `std::process::Command`, wait, and return the child's exit code through `AppError`/the dispatch path.

### Step 4: Wrapper-verb stubs + dispatch + main

Add `commands/dispatch.rs` (routes wrapper verbs vs. `External` passthrough) and stub handlers for `version`/`completion`/`config`/`doctor`/`init`. Wire `main.rs`: `parse → init logging → build AppContext → dispatch → map AppError to exit code`. Keep `main.rs` ≤120 LOC.

### Final Step: Update the queue

1. In this plan's `queue-rounds.yaml`, set this round's (`item: clap-passthrough-and-minimal-spawn`) `status` to `done`.

## Acceptance Criteria

- [ ] `claude-session --help` shows wrapper verbs + global flags without erroring on unknown native `claude` flags.
- [ ] An unknown subcommand/flag (e.g. `claude-session --print "hi"`) forwards verbatim to the real `claude` (verified against a stubbed child binary in an `assert_cmd` test).
- [ ] `claude-session version` prints claude-session's version plus the resolved child path+version.
- [ ] `-V` is wrapper-owned and does not shadow a child flag; `--` separator works.
- [ ] The wrapper exit code equals the child's exit code.
- [ ] This plan's `queue-rounds.yaml` shows round `clap-passthrough-and-minimal-spawn` as `done`.

## Next Round

Round 4 (`docs-adr-and-quality-gates`) establishes the Diátaxis `docs/` tree, the first ADRs, the `justfile`→pre-commit quality gates, `deny.toml`, and the `CLAUDE.md`→`AGENTS.md` SoT — closing out the foundation.
