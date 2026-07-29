# Docs & Hardening R3: Completions, Version & Man Pages

> Plan: cs-docs-hardening | Round: 3 of 4 | Complexity: M | Executor: prex (EF 1.5) | Generated: 2026-06-19 | Repo: repository root

## Context

With the full flag/verb surface settled, `claude-session` finalizes its shell completions, the `version` verb, and man pages. Completions must reflect only the wrapper-owned grammar (passthrough args are opaque). This round wires `clap_complete` and `clap_mangen` and confirms the `version` verb. `cs-foundation` shipped `completion`/`version` stubs; the feature plans finalized the flag set.

## Previous Rounds

`cs-foundation`: `completion`/`version` stubs + clap skeleton. Feature plans: the final flag/verb set (`account`, `config`, `profile`, global flags). Expect all to exist.

## Scope of This Round

- IN scope: finalize `commands/completion.rs` to generate shell completions via `clap_complete` for the wrapper-owned grammar (bash/zsh/fish); confirm `commands/version.rs` (our version + resolved child path + `claude --version`); add `clap_mangen` man-page generation (a `man`/`--man` path or build artifact); ensure completions/man pages cover the final wrapper flags and do NOT attempt to complete passthrough child args.
- OUT of scope: ADR closeout/release hardening (round 4); behavior changes to features.

## Current State

### Key Files

- `src/commands/completion.rs` — finalize.
- `src/commands/version.rs` — confirm child path/version.
- `Cargo.toml` — ensure `clap_complete`, `clap_mangen` present (add via `cargo add` if missing; never hand-edit `[dependencies]`).

### Existing Patterns

`clap_complete` and `clap_mangen` are on the reviewed list in `docs/reference/dependencies.md` as deferred crates unlocked by exactly this round; add them with `cargo add`.

The rule that decides the scope of both artifacts is in `docs/reference/cli-surface.md`: **completions and man pages cover the wrapper's grammar only.** Child arguments stay opaque, because tracking the child's flag list is precisely the coupling the passthrough contract exists to avoid. Both are generated from the same parser definition as `--help`; a hand-maintained flag table is forbidden.

## Implementation Steps

### First Step: Mark this round as started

In this plan's `queue-rounds.yaml`, set this round's (`item: completions-and-manpages`) `status` to `doing`.

### Step 1: Completions

Finalize `commands/completion.rs` to emit `clap_complete` completions (bash/zsh/fish) for the wrapper-owned grammar.

### Step 2: version

Confirm `commands/version.rs` prints our version + resolved child path + `claude --version`.

### Step 3: Man pages

Add `clap_mangen` man-page generation for the wrapper grammar.

### Step 4: Tests

Smoke-test completion + man generation (non-empty, valid for a shell) via `assert_cmd`.

### Final Step: Update the queue

1. In this plan's `queue-rounds.yaml`, set this round's (`item: completions-and-manpages`) `status` to `done`.

## Acceptance Criteria

- [ ] `claude-session completion <shell>` emits valid completions for the wrapper-owned grammar only.
- [ ] `claude-session version` prints our version + resolved child path + `claude --version`.
- [ ] `clap_mangen` man-page generation works.
- [ ] Completions/man pages do not attempt to complete passthrough child args.
- [ ] `assert_cmd` smoke tests pass.
- [ ] This plan's `queue-rounds.yaml` shows round `completions-and-manpages` as `done`.

## Next Round

Round 4 (`adr-closeout-and-release`) transitions ADRs to Implemented and does release hardening (deny/ audit clean, user README, install notes, final integration pass).
