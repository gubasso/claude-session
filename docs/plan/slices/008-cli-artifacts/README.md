# 008 — CLI artifacts

## Goal

Prove completions and man pages from the parser tree once the account MVP supplies the first feature namespace.

## Appetite

1 implementation session.

## Core

One parser tree generates help, completions, and man pages for wrapper grammar only, and later feature rungs regenerate from that same source.

## In scope

- Every documented completion shell, including the account MVP grammar.
- The JSON version form.
- Man-page generation from the parser tree, emitted to standard output.
- The repeatable regeneration and smoke-test path that later grammar-owning feature rungs invoke as tail work.

## Out of scope

- Completion or documentation of native child arguments.
- Grammar for a feature that has not landed.
- Hand-maintained or checked-in copies that can drift from the parser.

## Governed by

- [AGENTS.md](../../../../AGENTS.md)
- [CLI surface](../../../reference/cli-surface.md)
- [Dependencies](../../../reference/dependencies.md)
- [Testing and quality](../../../reference/testing-and-quality.md)
- [ADR-0016](../../../decisions/ADR-0016-ship-man-pages.md)
- [ADR-0086](../../../decisions/ADR-0086-emit-one-man-page-to-standard-output.md)

## Acceptance

- When a documented shell is selected, the wrapper shall emit nonempty completions derived from its parser. -> cli_artifacts::completions_cover_every_documented_shell
- When the account MVP grammar is final, generated completions and man pages shall include its wrapper-owned commands and no unimplemented feature grammar. -> cli_artifacts::artifacts_carry_the_account_grammar_and_no_unimplemented_verb
- When version output is requested, the wrapper shall report its version and the documented child-resolution state in text or JSON. -> output::version_json_reports_resolution_states
- When a man page is generated, the wrapper shall derive it from the same parser tree that produces help and completions. -> cli_artifacts::man_derives_the_root_page_from_the_parser_tree
- If child argv follows the wrapper grammar, then generated artifacts shall leave it opaque. -> cli_artifacts::generated_artifacts_leave_child_argv_opaque

## Rabbit holes

- Native child completions; escape: stop at the wrapper's passthrough boundary.
- Treating every later grammar change as a new artifact slice; escape: keep generation reusable and let the owning feature rung rerun it.

## Done when

Targeted account-grammar completion, version, man-page, and child-opacity smoke checks pass under `cargo nextest`, and `just hooks` is green.

## Revisions

None.
