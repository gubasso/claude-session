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
- Output-directory man-page generation from the parser tree.
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

## Acceptance

- When a documented shell is selected, the wrapper shall emit nonempty completions derived from its parser.
- When the account MVP grammar is final, generated completions and man pages shall include its wrapper-owned commands and no unimplemented feature grammar.
- When version output is requested, the wrapper shall report its version and the documented child-resolution state in text or JSON.
- When man pages are generated, the wrapper shall derive them from the same parser and honor the documented output destination.
- If child argv follows the wrapper grammar, then generated artifacts shall leave it opaque.

## Rabbit holes

- Native child completions; escape: stop at the wrapper's passthrough boundary.
- Treating every later grammar change as a new artifact slice; escape: keep generation reusable and let the owning feature rung rerun it.

## Done when

Targeted account-grammar completion, version, man-page, output-destination, and child-opacity smoke checks pass under `cargo nextest`, and `just hooks` is green.

## Revisions

None.
