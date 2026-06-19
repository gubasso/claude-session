# Foundation R2: Errors, Logging, Context, Config

> Plan: cs-foundation | Round: 2 of 4 | Complexity: L | Executor: prex (EF 1.5) | Generated: 2026-06-19 |
> Repo: /workspaces/claude-session

## Context

`claude-session` wraps `claude` and must follow the Rust cli-spec: typed errors per layer, a mandatory
exit-code matrix, tracing logging to XDG state, output discipline through a single writer, an
immutable resolved config, and an `AppContext` carrying process state. Round 1 produced a compiling
canonical module tree (`error.rs`, `logging.rs`, `ui.rs`, `context.rs`, `config.rs` exist as
placeholders) with blessed deps and strict lints. This round makes those modules real. They are
cross-cutting contracts every feature plan consumes, so they must be correct and tested now.

## Previous Rounds

Round 1 created `Cargo.toml` (blessed deps, strict lints, release profile, `rust-version`), pinned
toolchain components, and the empty canonical module tree, with a minimal `main.rs`. Expect
`src/error.rs`, `src/logging.rs`, `src/ui.rs`, `src/context.rs`, `src/config.rs` to exist as
`pub(crate)` placeholders and `cargo check` to pass.

## Scope of This Round

- IN scope: `error.rs` (`AppError` thiserror enum + `exit_code(&self) -> u8` exhaustive sysexits map +
  mandatory matrix unit test) and per-layer error skeletons (`DomainError`, `<Sys>AdapterError` with
  `#[from] std::io::Error`, `ServiceError`); `logging.rs` (one tracing-subscriber installed from
  `main`, non-blocking file sink to `$XDG_STATE_HOME/claude-session/claude-session.log` + optional
  stderr mirror, `RUST_LOG`-honoring, verbosity `-v/-vv/-vvv`/`-q`); `ui/` (single `Ui` writer: stdout
  = result, stderr = diagnostics, `NO_COLOR`/`FORCE_COLOR`); `context.rs` (`AppContext` immutable);
  `config/` (figment loader + immutable `Config` with `#[serde(deny_unknown_fields)]`, XDG paths via
  `directories`).
- OUT of scope: clap parsing, dispatch, command logic, child spawning (round 3); feature modules.

## Current State

### Key Files

- `/workspaces/claude-session/src/error.rs` — placeholder; becomes the `AppError` home.
- `/workspaces/claude-session/src/logging.rs` — placeholder; becomes the subscriber installer.
- `/workspaces/claude-session/src/ui.rs` (+ `src/ui/`) — placeholder; becomes the single output writer.
- `/workspaces/claude-session/src/context.rs` — placeholder; becomes `AppContext`.
- `/workspaces/claude-session/src/config.rs` (+ `src/config/`) — placeholder; becomes figment loader.

### Existing Patterns

Sysexits map (`cli-design/02-error-messages.md`): `0` ok, `64` usage, `65` dataerr, `66` noinput, `69`
unavail, `70` software, `74` ioerr, `75` auth/tempfail, `77` noperm, `78` config, `126` not-executable,
`127` child-not-found; signal deaths `128+N` (SIGINT→130, SIGTERM→143). Error messages are four-part
(What/Where/Why/Hint) with a stable machine-matchable `err.kind=<CamelCase>`. Logging: each record is
one structured line (`ts` ISO-8601 UTC ms, `level` lowercase, `target`, `op`/`msg`); verbosity
none=warn, `-v`=info, `-vv`=debug, `-vvv`=trace, `-q`=error-only; reuse `RUST_LOG` (do not invent
`CLAUDE_SESSION_LOG`). Resolve the log path in `main` BEFORE installing the subscriber, via
`directories::ProjectDirs::state_dir()`.

## Implementation Steps

### First Step: Mark this round as started

In this plan's `queue-rounds.yaml`, set this round's (`item: errors-logging-context-config`) `status`
to `doing`.

### Step 1: Error layers + exit-code matrix

In `error.rs`, define `AppError` (thiserror) with variants for the foreseeable failure classes (usage,
config, io, child-not-found, not-executable, auth, software). Implement
`pub(crate) fn exit_code(&self) -> u8` matching **every** variant explicitly to a sysexits code — NO
catch-all `_ => 1`. Add per-layer error skeletons in `domain.rs`/`adapters.rs`/`services.rs`
(`DomainError`, `<Sys>AdapterError` with `#[from] std::io::Error`, `ServiceError`) and `#[from]`
conversions feeding `AppError`. Write the **mandatory matrix unit test** asserting a code for every
variant (so a future variant without a code fails the build).

### Step 2: Logging

In `logging.rs`, install exactly one tracing-subscriber from a function called by `main`: a
non-blocking `tracing-appender` file sink at `$XDG_STATE_HOME/claude-session/claude-session.log`
(resolve via `directories`), plus an optional stderr mirror gated by verbosity, honoring `RUST_LOG`.

### Step 3: Single-writer UI

In `ui/`, define a `Ui` writer that owns ALL terminal output: stdout for the result (text or `--json`),
stderr for prompts/progress/status/warnings/errors. Respect `NO_COLOR`/`FORCE_COLOR`.

### Step 4: AppContext + Config

In `context.rs`, define an immutable `AppContext` (resolved paths, verbosity, format, a lazy child
resolver placeholder). In `config/`, build the figment loader (`env` + file) producing one immutable
`Config` with `#[serde(deny_unknown_fields)]`, precedence `cli > env > project > user > defaults`, env
prefix `CLAUDE_SESSION_` (nested `__`). XDG: user config at `~/.config/claude-session`;
data/state/cache/runtime per spec.

### Final Step: Update the queue

1. In this plan's `queue-rounds.yaml`, set this round's (`item: errors-logging-context-config`)
   `status` to `done`.

## Acceptance Criteria

- [ ] `AppError::exit_code()` covers every variant with NO catch-all; the matrix test passes and would
      fail if a variant lacked a code.
- [ ] Logging installs one subscriber, writes to `$XDG_STATE_HOME/claude-session/claude-session.log`,
      and honors `RUST_LOG` + `-v/-vv/-vvv/-q`.
- [ ] All terminal writes route through `Ui`; stdout carries only the result.
- [ ] `Config` is immutable, uses `deny_unknown_fields`, and resolves XDG paths via `directories`.
- [ ] `cargo nextest run` (pre-commit profile) passes.
- [ ] This plan's `queue-rounds.yaml` shows round `errors-logging-context-config` as `done`.

## Next Round

Round 3 (`clap-passthrough-and-minimal-spawn`) builds the root clap parser with verbatim external
passthrough, the intrinsic wrapper-verb stubs, argv normalization, a minimal inherited-env child spawn
so passthrough is demonstrable, and the `main.rs` dispatch wiring consuming this round's plumbing.
