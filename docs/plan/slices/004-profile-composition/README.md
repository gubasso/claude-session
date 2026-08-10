# 004 — Profile composition

## Goal

Let a user launch `claude-session --profile=dev` from one JSON piece and list the profiles available to select.

## Appetite

2 implementation sessions.

## Core

Identical one-piece inputs reuse one immutable byte-identical settings entry, and changed inputs name a new entry.

## In scope

- Resolution of a profile containing exactly one JSON piece.
- Content-addressed settings generation and the profile, path, digest, and piece fields of its provenance sidecar.
- Native `--settings` prefix injection while preserving every user-supplied child argument.
- The `profile` listing report in human and verb-level JSON forms.
- Single-piece profile entries in the doctor catalog, with their feature-owned probes and remediation.
- Regenerated completion and man-page coverage plus user documentation and per-rung release gates for `0.4.0`.

## Out of scope

- Ordered multi-piece merging, per-key array strategies, and contributor-chain provenance; slice 014 owns them.
- The `config` report, schema-aware validation, generated examples, and freshness tooling; slice 014 owns them.
- Managing child credentials or treating the composed document as the child's entire effective configuration.

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

- When a selected profile names one valid piece, the wrapper shall produce deterministic settings bytes and a provenance sidecar naming that profile and piece.
- When identical one-piece inputs are selected again, the wrapper shall reuse the complete immutable entry without rewriting it.
- When the profile bytes, resolved profile path, piece bytes, or resolved piece path change, the wrapper shall name a different entry.
- When a one-piece profile launch succeeds, the wrapper shall prepend the native `--settings` pair and preserve the user's child argv as an untouched suffix.
- When profiles are listed, the wrapper shall report the available profile names without claiming unimplemented multi-piece behavior.
- When the profile MVP lands, its doctor entries, generated CLI artifacts, user documentation, and `0.4.0` release gates shall agree with the implemented grammar.

## Rabbit holes

- Building a merge engine for one piece; escape: pass the single parsed document through deterministic generation and defer composition depth to slice 014.
- Mtime freshness; escape: content-addressed names make changed inputs name a new entry.

## Done when

Targeted profile resolution, entry-key, immutability, provenance, argv-prefix, listing, doctor-catalog, and generated-artifact tests pass under `cargo nextest`, the `0.4.0` rung documentation is honest, and `just hooks` is green.

## Revisions

None.
