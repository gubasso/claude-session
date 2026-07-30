# Foundation R1: Crate Manifest & Canonical Module Tree

> Plan: cs-foundation | Round: 1 of 4 | Complexity: L | Executor: prex (EF 1.5) | Generated: 2026-06-19 | Repo: repository root

## Context

`claude-session` is a from-scratch Rust CLI wrapping the `claude` command. The hard contract: never break native passthrough, stay XDG-compliant, stay self-contained. The crate today is a stub: `src/main.rs` prints "Hello, world!"; `Cargo.toml` has `name = "claude-session"`, `version = "0.1.0"`, `edition = "2024"`, empty `[dependencies]`; `rust-toolchain.toml` pins `channel = "stable"`. The surrounding project is fully wired — pre-commit, `justfile`, `deny.toml`, `flake.nix`, CI, and a populated `docs/` tree carrying the specifications this round implements against. This round establishes the manifest (first dependencies, strict lints, release profile, concrete `rust-version`) and the canonical empty module tree so `cargo check` passes — the substrate every later round wires into.

## Previous Rounds

This is the first round — no prior rounds.

## Scope of This Round

- IN scope: rewrite `Cargo.toml` (blessed deps, `[lints.rust]`/`[lints.clippy]`, `[[bin]]`, `[profile.release]`, `license`, `description`, concrete `rust-version`); add `components = ["rustfmt","clippy"]` to `rust-toolchain.toml`; create the empty canonical module tree (`src/cli.rs`, `src/commands.rs`, `src/domain.rs`, `src/services.rs`, `src/adapters.rs`, `src/config.rs`, `src/context.rs`, `src/error.rs`, `src/logging.rs`, `src/ui.rs`, `src/util.rs`) with minimal placeholder content so `cargo check` succeeds; reduce `src/main.rs` to a minimal `fn main()`.
- OUT of scope: real error variants, logging install, clap parsing, command logic (later rounds).

## Current State

### Key Files

- `Cargo.toml` — `[package] name = "claude-session" version = "0.1.0"
  edition = "2024"`; empty `[dependencies]`.
- `src/main.rs` — `fn main() { println!("Hello, world!"); }`.
- `rust-toolchain.toml` — `[toolchain]` / `channel = "stable"`.
- `.gitignore` — `/target`.

### Existing Patterns

The reviewed dependency set, the deferred set, and the ruled-out set with reasons are specified in `docs/reference/dependencies.md`. That page is candidates, not decisions: **add only what the foundation actually uses now**, since `cargo machete` fails on a declared-but-unused dependency. Note the deferred crates in a comment for the rounds that will need them. Two crates on that page need a deliberate call here: `tokio` is probably unnecessary — a wrapper that spawns one child and waits has no need for an async runtime — and `camino` applies only where paths are known-UTF-8, never at the argv or environment boundary.

Strict lints are specified in `docs/reference/coding-conventions.md`: `[lints.rust] unsafe_code = "forbid"`, `unused_must_use = "deny"`, `unreachable_pub = "warn"`; `[lints.clippy]` `all`/`pedantic`/`nursery` = warn (priority -1), `unwrap_used`/`expect_used` = warn. Release profile: `lto = "thin"`, `codegen-units = 1`, `strip = "symbols"`. The module tree and its per-directory prohibitions are in `docs/explanation/architecture.md`; the `foo.rs` + `foo/` form and the `pub(crate)` default are in `docs/reference/coding-conventions.md`.

## Implementation Steps

### First Step: Mark this round as started

In this plan's `queue-rounds.yaml`, set this round's (`item: crate-manifest-and-module-tree`) `status` to `doing`.

### Step 1: Rewrite `Cargo.toml`

Hand-write the static manifest sections: `rust-version` (current stable), `license = "MIT OR Apache-2.0"`, a `description`, a `[[bin]]` (`name = "claude-session"`, `path = "src/main.rs"`), `[profile.release]`, and the `[lints.rust]`/`[lints.clippy]` blocks. Then add every dependency this round actually uses with **`cargo add`** (e.g. `cargo add clap --features derive,env,wrap_help`) — **NEVER by hand-editing `[dependencies]` or writing a version string** — so cargo resolves the graph and updates `Cargo.lock`. This rule is project-wide; see `docs/decisions/0009-blessed-dependency-set.md`. Do not add anything on the ruled-out list in `docs/reference/dependencies.md`, and do not pre-add deferred crates — `cargo machete` fails on an unused dependency. Deviate from `cargo add` only with a documented exception noted at the dependency.

### Step 2: Pin toolchain components

Edit `rust-toolchain.toml` to add `components = ["rustfmt", "clippy"]`.

### Step 3: Create the canonical module tree

Create placeholder modules (post-2018 form, `foo.rs` + optional `foo/`): `src/cli.rs`, `src/commands.rs`, `src/domain.rs`, `src/services.rs`, `src/adapters.rs`, `src/config.rs`, `src/context.rs`, `src/error.rs`, `src/logging.rs`, `src/ui.rs`, `src/util.rs`. Each default visibility `pub(crate)`; add `mod` declarations in `main.rs`; keep placeholders minimal (a doc comment

- maybe one marker type) so lints pass.

### Step 4: Minimal `main.rs`

Reduce `src/main.rs` to declare the modules and a minimal `fn main()` that returns/exits cleanly (placeholder; real wiring lands R2–R3). Keep ≤120 LOC.

### Final Step: Update the queue

1. In this plan's `queue-rounds.yaml`, set this round's (`item: crate-manifest-and-module-tree`) `status` to `done`.

## Acceptance Criteria

- [ ] `cargo check` succeeds with the new manifest and module tree.
- [ ] `cargo clippy` runs with the strict lint blocks active (no errors; warnings ok in placeholders).
- [ ] `Cargo.toml` has `[[bin]]`, `[profile.release]`, concrete `rust-version`, `license`, lint blocks; no forbidden deps.
- [ ] Every dependency was added via `cargo add` (latest compatible versions resolved); `Cargo.lock` is updated and committed. No dependency version strings were hand-written (barring a documented exception).
- [ ] `rust-toolchain.toml` lists `components = ["rustfmt", "clippy"]`.
- [ ] The canonical module tree exists and `src/main.rs` is ≤120 LOC.
- [ ] This plan's `queue-rounds.yaml` shows round `crate-manifest-and-module-tree` as `done`.

## Next Round

Round 2 (`errors-logging-context-config`) fills the plumbing: thiserror layers + the mandatory exit-code matrix test, tracing logging to `$XDG_STATE_HOME`, the single-writer `ui`, the immutable `AppContext`, and the figment `Config` loader.
