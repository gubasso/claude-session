# Native repository gates

## Goal

Move the repository's own contract checks from shell scripts into the test suite.

## Appetite

1 implementation session.

## Core

Four checks become one integration target with floor guards and negative fixtures, and the scripts they replace are deleted.

## In scope

- The ADR contract, plan-zone contract, decorative-emphasis, and Rust-boundary checks as one `tests/repo_contracts` target.
- A self-integrity proof for each gate: a floor, named sentinels, and negative fixtures driven from doctored literals.
- Deletion of the four scripts and their four hooks.
- Reconciling the reference and the boundary-lint record with the mechanism that now enforces them.

## Out of scope

- `scripts/check-acceptance-tests`, whose oracle is `cargo nextest list` and cannot run inside a cargo test.
- The publish and release scripts.
- Any new dependency; the gates use the standard library and the crates already reviewed.
- A general Markdown parser.

## Governed by

- [AGENTS.md](../../../../AGENTS.md)
- [Testing and quality](../../../reference/testing-and-quality.md)
- [Coding conventions](../../../reference/coding-conventions.md)
- [Dependencies](../../../reference/dependencies.md)
- [ADR-0074](../../../decisions/ADR-0074-enforce-boundary-rules-with-clippy-configuration.md)
- [ADR-0076](../../../decisions/ADR-0076-cap-filled-adrs-at-350-words.md)
- [ADR-0077](../../../decisions/ADR-0077-add-a-plan-zone-to-reader-need-docs.md)
- [ADR-0078](../../../decisions/ADR-0078-adopt-the-seven-state-decision-lifecycle.md)

## Acceptance

- When a decision record breaks its id sequence, shape, status vocabulary, relationship links, or word cap, the suite shall fail naming the file. -> repo_contracts::adrs::every_adr_satisfies_the_contract
- When a milestone row, slice entry, task list, or question block breaks the plan-zone contract, the suite shall fail naming the file. -> repo_contracts::plan_zone::the_plan_zone_satisfies_the_contract
- When decorative bold or italic prose appears in any Markdown file outside code and outside build output, the suite shall fail naming the file and line. -> repo_contracts::emphasis::every_markdown_file_is_free_of_decorative_emphasis
- When `src/domain` names an adapter or a service, the suite shall fail. -> repo_contracts::boundaries::the_domain_layer_imports_no_adapter_or_service
- When any source under `src` names `xtask` as a whole word, the suite shall fail. -> repo_contracts::boundaries::nothing_under_src_imports_xtask
- When the shipped manifest lists a development-tooling crate, the suite shall fail. -> repo_contracts::boundaries::the_shipped_manifest_lists_no_tooling_crate
- If a documentation walk returns fewer files than its floor, then the suite shall fail rather than report success. -> repo_contracts::emphasis::the_walk_reaches_the_whole_repository
- The suite shall add no dependency outside the standard library and the crates the dependency reference already reviews.

## Rabbit holes

- Writing a general Markdown parser; escape: recognize only the dialect the formatter emits and fail closed on anything else.
- Admitting a walker or regular-expression crate to shorten the port; escape: keep the hand-written scanners and cover each with a table test.
- Migrating `scripts/check-acceptance-tests`; escape: its oracle is `cargo nextest list`, so it stays shell and keeps its hook.
- Strengthening a rule while porting it; escape: land the port first, then change behaviour in a separate change that says so.

## Done when

The four gates run as `repo_contracts` integration tests with floor guards and negative fixtures, the four scripts and their hooks are gone, the reference and ADR-0074 name the new mechanism, and `just hooks` is green.

## Revisions

- 2026-08-06: Split the boundary acceptance into one assertion per rule and added a floor-guard assertion, so every line names one resolvable test.
- 2026-08-06: A mutation differential doctored 43 real repository files and compared both implementations. Every verdict agreed. Two behaviour changes are deliberate: a `[dependencies.name]` subtable is now caught, which the manifest scan missed, and a `##` heading inside a fenced example no longer breaks the record heading list, which the scan reported as drift. The commit-stage coverage of the boundary and emphasis rules moves to push, which the lane rule requires.
