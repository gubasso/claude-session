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
- Structural validation of what composition owns, forwarding every child settings key unmodelled.
- `config` reports and the full profile report behavior in human and verb-level JSON forms.
- Generated configuration examples, schemas, round-trip checks, and freshness tooling.
- Full-composition entries in the doctor catalog, with their feature-owned probes and remediation.
- Regenerated completion and man-page coverage plus user documentation and per-rung release gates for `0.5.0`.

## Out of scope

- Managing child credentials or trust state.
- Treating the composed document as the child's entire effective configuration.
- Any validation of the child's own settings keys, in either direction.

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

- When ordered pieces are composed, the wrapper shall apply recursive object merge, scalar last-wins, and the selected per-key array strategy deterministically. -> session_storage::ordered_pieces_compose_by_recursive_object_merge
- When more than one piece touches a key, the wrapper shall preserve the ordered contributor chain in provenance. -> session_storage::a_concat_strategy_appends_in_layer_order_with_every_contributor
- If a strategy pointer is invalid or cannot apply, then the wrapper shall reject the composition with its path and source. -> session_storage::an_invalid_strategy_pointer_is_rejected_with_its_path_and_source
- When a merged document contains a key the wrapper does not model, the wrapper shall compose and forward it unchanged and report nothing about it. -> session_storage::a_key_the_wrapper_does_not_model_is_composed_and_forwarded_unchanged
- When `config` or profile data is reported, the wrapper shall expose the documented resolved inputs, entry, defects, and provenance consistently in human and JSON forms. -> configuration::config_json_and_human_reports_carry_the_same_facts
- When generated examples and schemas are checked, the repository shall prove that they round-trip through their owning types and match fresh generator output. -> generated_examples::every_generated_artifact_matches_fresh_generator_output
- When full composition lands, its doctor entries, generated CLI artifacts, user documentation, and `0.5.0` release gates shall agree with the implemented grammar. -> doctor::full_profile_composition_documentation_matches_the_implemented_grammar

## Rabbit holes

- Modelling the child's settings schema at all; escape: generate the document and let the child validate it.
- Mtime freshness; escape: content-addressed names make changed inputs name a new entry.
- Adding merge surfaces beyond current profile inputs; escape: implement only the documented strategy table and reports.

## Done when

Targeted merge, provenance, validation, generation, command, freshness, doctor-catalog, and generated-artifact tests pass under `cargo nextest`, the `0.5.0` rung documentation is honest, and `just hooks` is green.

## Revisions

- 2026-08-12: The wrapper models the child's settings keys not at all, rather than permissively with a warning. The recognized-key table it warned against covered fourteen of sixty-five or more documented keys and had no way to close that gap, so it warned about valid settings more often than about mistakes. The acceptance line that bound the warning now binds the forwarding it replaced, and the type-conflict refusal reaches inside matched `merge-by-key` elements, which is what validating the composition rather than the schema means. See [ADR-0088](../../../decisions/ADR-0088-model-nothing-the-child-already-owns.md).
