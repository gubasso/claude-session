# 004 — Profile composition

## Goal

Let a selected YAML profile compose ordered JSON pieces into the native settings document used by a launch.

## Appetite

4 implementation sessions.

## Core

Identical inputs yield one immutable byte-identical output and changed inputs yield a new entry.

## In scope

- Per-key merge strategies and complete contributor provenance.
- Schema-aware structural validation that permits unknown native keys.
- `config` and `profile` reports with verb-level JSON.
- Generated examples and schema freshness tooling.

## Out of scope

- Managing child credentials or trust state.
- Treating the composed document as the child's entire effective configuration.

## Governed by

- [AGENTS.md](../../../../AGENTS.md)
- [Architecture](../../../explanation/architecture.md)
- [Session isolation](../../../explanation/session-isolation.md)
- [Configuration](../../../reference/configuration.md)
- [XDG storage](../../../reference/xdg-storage.md)
- [CLI surface](../../../reference/cli-surface.md)
- [Logging and output](../../../reference/logging-and-output.md)
- [Presentation](../../../reference/presentation.md)
- [Coding conventions](../../../reference/coding-conventions.md)
- [Dependencies](../../../reference/dependencies.md)
- [Testing and quality](../../../reference/testing-and-quality.md)

## Acceptance

- When ordered pieces are composed, the wrapper shall apply recursive object merge, scalar last-wins, and the selected per-key array strategy deterministically.
- When more than one piece touches a key, the wrapper shall preserve the ordered contributor chain in provenance.
- If a strategy pointer is invalid or cannot apply, then the wrapper shall reject the composition with its path and source.
- When a merged document contains an unknown native key, the wrapper shall preserve it and report a warning with provenance.
- When composition succeeds, the wrapper shall pass the immutable entry through the native `--settings` prefix.

## Rabbit holes

- Strictly rejecting the complete native schema; escape: Q-003 must exit by measurement first.
- Mtime freshness; escape: content-addressed names make changed inputs name a new entry.

## Done when

Targeted model, merge, provenance, generation, command, and freshness tests pass under `cargo nextest`.

## Revisions

None.
