# Foundation R1: Crate Manifest & Canonical Module Tree

> Plan: cs-foundation | Round: 1 of 4 | Complexity: L | Executor: prex (EF 1.5) | Generated: 2026-06-19 | Repo: /workspaces/claude-session

## Context

`claude-session` is a from-scratch Rust CLI wrapping the `claude` command (baseline v2.1.183). The hard contract: never break native passthrough, stay XDG-compliant and self-contained, follow the Rust cli-spec. The repo today is a bare scaffold: `src/main.rs` prints "Hello, world!"; `Cargo.toml` has `name = "claude-session"`, `version = "0.1.0"`, `edition = "2024"`, empty `[dependencies]`; `rust-toolchain.toml` pins `channel = "stable"`; `.gitignore` is `/target`; pre-commit tooling is already wired (`.pre-commit-config.yaml`, `committed.toml`, `.config/nextest.toml`, `.editorconfig`, `Cargo.lock`). This round establishes the manifest (blessed deps, strict lints, release profile, concrete `rust-version`) and the canonical empty module tree so `cargo check` passes — the substrate every later round wires into.

## Previous Rounds

This is the first round — no prior rounds.

## Scope of This Round

- IN scope: rewrite `Cargo.toml` (blessed deps, `[lints.rust]`/`[lints.clippy]`, `[[bin]]`, `[profile.release]`, `license`, `description`, concrete `rust-version`); add `components = ["rustfmt","clippy"]` to `rust-toolchain.toml`; create the empty canonical module tree (`src/cli.rs`, `src/commands.rs`, `src/domain.rs`, `src/services.rs`, `src/adapters.rs`, `src/config.rs`, `src/context.rs`, `src/error.rs`, `src/logging.rs`, `src/ui.rs`, `src/util.rs`) with minimal placeholder content so `cargo check` succeeds; reduce `src/main.rs` to a minimal `fn main()`.
- OUT of scope: real error variants, logging install, clap parsing, command logic (later rounds).

## Current State

### Key Files

- `/workspaces/claude-session/Cargo.toml` — `[package] name = "claude-session" version = "0.1.0"
  edition = "2024"`; empty `[dependencies]`.
- `/workspaces/claude-session/src/main.rs` — `fn main() { println!("Hello, world!"); }`.
- `/workspaces/claude-session/rust-toolchain.toml` — `[toolchain]` / `channel = "stable"`.
- `/workspaces/claude-session/.gitignore` — `/target`.

### Existing Patterns

Blessed dependency set (`rust/cli-spec/07-dependencies.md`): `clap` (`derive`,`env`,`wrap_help`), `clap_complete`, `clap_mangen`, `anyhow`, `thiserror`, `tracing`, `tracing-subscriber` (`env-filter`,`fmt`), `tracing-appender`, `serde` (`derive`), `serde_json`, `toml`, `figment` (`env`,`toml`), `directories`, `camino` (`serde1`), `tokio` (`rt`,`macros`). Add only what the foundation needs now; note deferred deps (`serde_yaml_ng` for manifests, `rustix`/`libc`/`signal-hook` for the runtime, `tempfile`, `which`) in comments for the consuming rounds. Strict lints (codex reference pattern, cli-spec `09-coding-style.md`): `[lints.rust] unsafe_code = "forbid"`, `unused_must_use = "deny"`, `unreachable_pub = "warn"`; `[lints.clippy]` `all`/`pedantic`/`nursery` = warn (priority -1), `unwrap_used`/`expect_used` = warn. Release profile: `lto = "thin"`, `codegen-units = 1`, `strip = "symbols"`.

## Implementation Steps

### First Step: Mark this round as started

In this plan's `queue-rounds.yaml`, set this round's (`item: crate-manifest-and-module-tree`) `status` to `doing`.

### Step 1: Rewrite `Cargo.toml`

Hand-write the static manifest sections: `rust-version` (current stable, e.g. `"1.85"`), `license = "MIT OR Apache-2.0"`, a `description`, a `[[bin]]` (`name = "claude-session"`, `path = "src/main.rs"`), `[profile.release]`, and the `[lints.rust]`/`[lints.clippy]` blocks. Then add every dependency the foundation needs now with **`cargo add`** (e.g. `cargo add clap --features derive,env,wrap_help`) — **NEVER by hand-editing `[dependencies]` or writing a version string** — so cargo resolves the graph, fetches the latest compatible version, and generates/updates `Cargo.lock`. This `cargo add` rule is project-wide and applies to every later round that introduces a dependency. Do NOT add `dirs`/`chrono`/`lazy_static`/`serde_yaml`/`env_logger`/ `structopt`. Deviate from `cargo add` only with a documented exception (known-broken latest or a required exact pin), noted in a manifest comment or ADR.

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
