# 014 — Full profile composition

## Goal

Extend the profile MVP into deterministic ordered composition with explainable merge behavior and maintained configuration artifacts.

## Appetite

2 implementation sessions.

## Core

Ordered inputs produce deterministic settings and complete contributor provenance under explicit per-key merge strategies.

## In scope

- Ordered multi-piece recursive merging, scalar last-wins behavior, and per-key array strategies.
- Complete per-key contributor provenance and actionable path-and-source failures.
- Schema-aware structural validation that preserves unknown native keys with provenance warnings.
- `config` reports and the full profile report behavior in human and verb-level JSON forms.
- Generated configuration examples, schemas, round-trip checks, and freshness tooling.
- Full-composition entries in the doctor catalog, with their feature-owned probes and remediation.
- Regenerated completion and man-page coverage plus user documentation and per-rung release gates for `0.5.0`.

## Out of scope

- Managing child credentials or trust state.
- Treating the composed document as the child's entire effective configuration.
- Strictly rejecting unknown native settings keys without the measurement required by Q-003.

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
- When `config` or profile data is reported, the wrapper shall expose the documented resolved inputs, entry, defects, and provenance consistently in human and JSON forms.
- When generated examples and schemas are checked, the repository shall prove that they round-trip through their owning types and match fresh generator output.
- When full composition lands, its doctor entries, generated CLI artifacts, user documentation, and `0.5.0` release gates shall agree with the implemented grammar.

## Rabbit holes

- Strictly rejecting the complete native schema; escape: Q-003 must exit by measurement first.
- Mtime freshness; escape: content-addressed names make changed inputs name a new entry.
- Adding merge surfaces beyond current profile inputs; escape: implement only the documented strategy table and reports.

## Done when

Targeted merge, provenance, validation, generation, command, freshness, doctor-catalog, and generated-artifact tests pass under `cargo nextest`, the `0.5.0` rung documentation is honest, and `just hooks` is green.

## Revisions

None.
