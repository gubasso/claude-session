# Native passthrough foundation

## Goal

Replace the placeholder with a working stock-compatible pass-through.

## Appetite

4 implementation sessions.

## Core

Unknown native argv is byte-preserved, the child runs, and its status is returned.

## In scope

- Error, logging, UI, context, and configuration plumbing.
- The complete wrapper-owned pre-split, flag denylist, and verb classification.
- The measured child inventory fixture and the collision audit that reads it.
- Version reporting for the wrapper and resolved child.
- Help and version composed with the child's, under the composed-output delimiter.
- Boundary, output, documentation, and quality gates.

## Out of scope

- Secure account/profile storage, robust signal supervision, authentication, and settings composition.
- Parsing or modelling the child's grammar.
- Declaring a verb whose behaviour a later slice specifies; until then its spelling stays passthrough.

## Governed by

- [AGENTS.md](../../../../AGENTS.md)
- [Architecture](../../../explanation/architecture.md)
- [Wrapper model](../../../explanation/wrapper-model.md)
- [CLI surface](../../../reference/cli-surface.md)
- [Process runtime](../../../reference/process-runtime.md)
- [Exit codes](../../../reference/exit-codes.md)
- [Logging and output](../../../reference/logging-and-output.md)
- [Configuration](../../../reference/configuration.md)
- [XDG storage](../../../reference/xdg-storage.md)
- [Coding conventions](../../../reference/coding-conventions.md)
- [Dependencies](../../../reference/dependencies.md)
- [Testing and quality](../../../reference/testing-and-quality.md)

## Acceptance

- When unknown native arguments are supplied, the wrapper shall forward their bytes, order, count, empty values, and separator unchanged. -> passthrough::golden_argv_preserves_bytes_order_count_and_empty_values
- When the child exits or is terminated, the wrapper shall return the same observable status. -> passthrough::child_exit_and_signal_status_are_preserved
- If the resolved child is missing or invalid, then the wrapper shall return the documented typed error without recursing. -> child_resolution::configured_child_is_terminal_when_missing
- While a wrapper verb is unimplemented, its spelling shall stay undeclared and reach the child unchanged. -> passthrough::unimplemented_verbs_reach_child
- When a claimed read-only spelling overlaps the child's, the wrapper shall emit its own output, the delimiter, then the child's bytes unchanged. -> output::claimed_read_only_surfaces_compose_child_bytes
- When a claimed spelling appears in the child inventory fixture without being named in the CLI surface, the build shall fail. -> collision_audit::claimed_flags_match_the_child_inventory

## Rabbit holes

- Full process supervision; escape: keep the minimal spawn seam replaceable for slice 003.
- Feature-complete verb handlers; escape: declare `version` and `help` only, and add each remaining verb in the slice that specifies it.

## Done when

The targeted `cargo nextest` and `assert_cmd` passthrough checks pass, followed by the repository documentation and quality hooks.

## Revisions

- 2026-08-06: Added resolver-backed acceptance test IDs after the slice gained real unit and integration tests.
