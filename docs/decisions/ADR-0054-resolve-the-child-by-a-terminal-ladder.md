# ADR-0054: Resolve the child by a terminal ladder

## Context and Problem Statement

The child-resolution ladder said "the first candidate that exists wins" while its own validation table made a non-existent path `ChildNotFound`. For a typo'd `child_bin` those give opposite answers: silently search `PATH`, or stop. The ladder also listed four rungs, two of which are the same configuration key at different precedence layers, and one of which no distribution ships.

## Considered Options

- A rung is consulted only when its source is absent; a present source is terminal
- Fall through to the next rung whenever a candidate fails to validate
- Prune the wrapper's own directory from `PATH` and keep searching, as pyenv does
- Add a dedicated executable directory ahead of `PATH`, as git's `GIT_EXEC_PATH` does

## Decision Outcome

Chosen option: a present source is terminal — falling through would run a different binary than the user named, which is the failure the ladder exists to prevent.

Two rungs, not four. `child_bin` is one configuration key whose layering [configuration](../reference/configuration.md#precedence) already owns, so naming its environment and file layers separately was a second source of truth. The bundled rung is dropped: nothing ships a `claude`, and a rung that never fires discriminates nothing ([ADR-0048](./ADR-0048-build-for-a-present-need.md), [ADR-0051](./ADR-0051-let-every-surface-element-discriminate.md)).

The `PATH` rung searches the inherited `PATH` for the fixed name `claude` and always spawns the absolute path it resolved, so `execvp` never runs a second search whose `errno` would be ambiguous. Mechanics are in [process runtime](../reference/process-runtime.md#child-resolution).

## Consequences

- Good: a broken override is reported against the path the user wrote, not silently replaced.
- Good: one owner for `child_bin`'s precedence.
- Bad: a user whose `child_bin` points at a since-removed install gets a failure rather than a working `PATH` fallback. That is the intent — the fallback is a `claude-session` that runs a binary its operator did not choose.
- Bad: vendoring a child later needs a superseding record, not an edit.

## Status

Implemented

Enacted by [`src/services/child.rs`](../../src/services/child.rs).
