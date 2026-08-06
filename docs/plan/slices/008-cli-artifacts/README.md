# 008 — CLI artifacts

<!-- markdownlint-configure-file { "MD043": { "headings": ["*","## Goal","## Appetite","## Core","## In scope","## Out of scope","## Governed by","## Acceptance","## Rabbit holes","## Done when","## Revisions"] } } -->

## Goal

Generate completions, version output, and man pages from the final wrapper grammar.

## Appetite

1 implementation session.

## Core

Generated artifacts cover wrapper-owned grammar only and never model child argv.

## In scope

- Every documented completion shell.
- The JSON version form.
- Output-directory man-page generation.

## Out of scope

- Completion or documentation of native child arguments.
- Hand-maintained grammar copies.

## Governed by

- [AGENTS.md](../../../../AGENTS.md)
- [CLI surface](../../../reference/cli-surface.md)
- [Dependencies](../../../reference/dependencies.md)
- [Testing and quality](../../../reference/testing-and-quality.md)
- [ADR-0016](../../../decisions/ADR-0016-ship-man-pages.md)

## Acceptance

- When a documented shell is selected, the wrapper shall emit nonempty completions derived from its parser.
- When version output is requested, the wrapper shall report its version and the documented child-resolution state in text or JSON.
- When man pages are generated, the wrapper shall derive them from the same parser and honor the documented output destination.
- If child argv follows the wrapper grammar, then generated artifacts shall leave it opaque.

## Rabbit holes

- Native child completions; escape: stop at the wrapper's passthrough boundary.
- Checked-in generated pages; escape: generate on demand unless the current owner requires shipping artifacts.

## Done when

Targeted completion, version, and man-page smoke checks pass under `cargo nextest`.

## Revisions

None.
