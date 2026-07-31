# Foundation R3: Clap Passthrough Skeleton & Minimal Spawn

> Plan: cs-foundation | Round: 3 of 4 | Complexity: L | Executor: prex (EF 1.5) | Generated: 2026-06-19 | Repo: repository root

## Context

`claude-session` MUST never break native `claude` passthrough: any unknown arg/flag/subcommand forwards verbatim (`OsString`-preserving) to the real `claude`, and a bare `claude-session ...` runs `claude ...`. Wrapper-owned global flags collide with native `claude` flags only where `docs/reference/cli-surface.md` names the overlap and states its reason; interception is leading-position only, and `--` always reaches the child spelling (ADR-0043, ADR-0044). This round builds the clap skeleton that routes wrapper-verbs vs. passthrough, the wrapper-verb stubs, and a **minimal** inherited-env child spawn so passthrough is demonstrable and testable now. The robust `Spawner` trait, full signal forwarding, exit-code mapping, recursion guard, and isolated child-env injection are deliberately deferred to the `cs-wrapper-runtime` plan. Rounds 1–2 produced the crate tree and the error/logging/context/config plumbing.

## Previous Rounds

Round 1: canonical module tree + manifest. Round 2: `AppError` + exit-code matrix, tracing logging, single-writer `Ui`, immutable `AppContext`, figment `Config`. Expect `error.rs`/`logging.rs`/`ui/`/ `context.rs`/`config/` to be real and tested.

## Scope of This Round

- IN scope: `cli/argv.rs`, the pure argv pre-split described below; `cli.rs` root `Cli` parser (`disable_version_flag` so `-V` is wrapper-owned) with a `GlobalArgs` flatten carrying the wrapper-owned flags specified in `docs/reference/cli-surface.md` (`--verbose` count — long form only, since the child spells `-v` as `--version`; `-q/--quiet`, `--config <path>`, `--dry-run`, plus `--account`, `--session`, `--profile` reserved as stubs for later plans; machine output is **not** here — `--json` is verb-level per ADR-0024) and a `Commands` enum covering exactly the verbs the table in `docs/reference/cli-surface.md` lists; a free `pub fn run(ctx, args) -> Result<(), AppError>` stub per verb reporting "not yet implemented" via `Ui`, except `version`, which prints our version plus the resolved child path and version in the shape that page specifies; a minimal `commands/pass_through.rs` that spawns the real `claude` with **inherited env** (no isolation) via `std::process::Command`, waits, and propagates the child exit code; `commands/dispatch.rs`; `main.rs` (`parse → init logging → AppContext → dispatch → exit-code map`, ≤120 LOC).
- OUT of scope: the hexagonal `Spawner` trait, signal forwarding, recursion guard, child env construction and `CLAUDE_CONFIG_DIR` injection (all in `cs-wrapper-runtime`); session dirs (`cs-isolation`); accounts (`cs-accounts-auth`); config composition (`cs-config-composition`).

## Current State

### Key Files

- `src/cli.rs` (+ `src/cli/`) — placeholder; becomes the root parser.
- `src/commands.rs` (+ `src/commands/`) — placeholder; becomes dispatch + pass_through + verb handlers.
- `src/main.rs` — minimal; becomes the real entrypoint.

### Existing Patterns

The wrapper grammar, the claimed-flag denylist, the verb list, and the parser shape are specified in `docs/reference/cli-surface.md`; the reasoning is in `docs/explanation/wrapper-model.md`.

**The parser cannot do this alone.** External-subcommand handling captures an unexpected _positional_ token; a leading unknown **flag** — which is the most common passthrough invocation, `claude-session --print "hi"` — is rejected as an unexpected argument before that handling applies, and relaxed hyphen settings do not rescue it because they apply to a declared value rather than to the top-level parse. Argv is therefore **pre-split before it reaches the parser**: a pure function consumes only tokens the denylist claims, stops at the first token that is not one or at `--`, and hands the parser a grammar in which every token is one it defines. When the remainder does not begin with a wrapper verb it is child argv and never touches the parser.

Argv layout is `claude-session [WRAPPER FLAGS] <verb> [--] [CHILD ARGS...]`, `--` is a hard sentinel, and forwarding preserves order, bytes, count, and empty arguments as `OsString`.

### Process model note

claude-session **spawn-and-waits** rather than `exec`s, for the supervision and post-flight obligations amended `docs/decisions/ADR-0004-spawn-and-wait-child-supervision.md` and `docs/reference/process-runtime.md` own. This round's minimal spawn does the simplest correct thing: inherit env, run `claude`, propagate exit code. `cs-wrapper-runtime` replaces it with the robust path.

## Implementation Steps

### First Step: Mark this round as started

In this plan's `queue-rounds.yaml`, set this round's (`item: clap-passthrough-and-minimal-spawn`) `status` to `doing`.

### Step 1: Root parser + GlobalArgs + Commands enum

Build `cli.rs` with the `Cli` struct, `GlobalArgs` (wrapper-owned, `global = true`), and the `Commands` enum covering the wrapper verbs. Claim `-V` via `disable_version_flag`, so `--version` can report both the wrapper and the resolved child. Reserve (but stub) `--account`, `--session`, `--profile`. The parser never sees child argv — the pre-split in step 2 keeps it away.

### Step 2: argv pre-split

Add `cli/argv.rs` with a **pure, total** pre-split over `Vec<OsString>`: consume leading tokens the wrapper's denylist claims and their values, stop at the first token that is not one or at `--`, and classify the remainder as either a wrapper verb (hand to the parser) or child argv (never hand to the parser).

**Normalize nothing.** Do not strip empty arguments — an empty string is a real argument and filtering it silently changes the user's command line. Do not reorder, deduplicate, or re-quote. Preserve `OsString` throughout. See `docs/decisions/ADR-0002-verbatim-argv-passthrough.md`.

Unit-test the pre-split directly against the golden-argv table in `docs/reference/testing-and-quality.md`: it is the single point where the passthrough contract can silently break.

### Step 3: Minimal pass-through spawn

In `commands/pass_through.rs`, resolve `claude` via a simple `which`/PATH lookup (full resolution chain arrives in `cs-wrapper-runtime`), spawn with inherited env via `std::process::Command`, wait, and return the child's exit code through `AppError`/the dispatch path.

### Step 4: Wrapper-verb stubs + dispatch + main

Add `commands/dispatch.rs` (routes wrapper verbs vs. `External` passthrough) and one stub handler per verb in the `docs/reference/cli-surface.md` table. That table is the closed list — a verb absent from it, including one a previous draft of this project reserved, is not stubbed here. Wire `main.rs`: `parse → init logging → build AppContext → dispatch → map AppError to exit code`. Keep `main.rs` ≤120 LOC.

### Final Step: Update the queue

1. In this plan's `queue-rounds.yaml`, set this round's (`item: clap-passthrough-and-minimal-spawn`) `status` to `done`.

## Acceptance Criteria

- [ ] `claude-session --help` shows wrapper verbs + global flags without erroring on unknown native `claude` flags.
- [ ] An unknown subcommand/flag (e.g. `claude-session --print "hi"`) forwards verbatim to the real `claude` (verified against a stubbed child binary in an `assert_cmd` test).
- [ ] `claude-session version` prints claude-session's version plus the resolved child path+version.
- [ ] `-V` is wrapper-owned and free of the child's inventory; every other overlap matches the documented child-status column; `--` separator works, and a second `--` reaches the child as an ordinary argument.
- [ ] The wrapper exit code equals the child's exit code.
- [ ] This plan's `queue-rounds.yaml` shows round `clap-passthrough-and-minimal-spawn` as `done`.

## Next Round

Round 4 (`docs-adr-and-quality-gates`) adds the output-ownership and dependency-direction lints and runs the conformance pass against the specifications — closing out the foundation.
