# 021 — Requested help completion

## Goal

Make requested help a result at every spelling the CLI surface names, so `doctor --help`, `version --help`, `help doctor`, and `help version` print on standard output and exit `0` instead of failing as malformed, with `doctor` carrying the child's own help after the composed-output delimiter.

## Appetite

1 implementation session.

## Core

Every spelling the requested-help contract names answers with the verb node's own help, on standard output, at exit `0`.

## In scope

- A per-verb help flag on `doctor` and on `version`, each with its own parser identifier, which is what the five verbs that already answer requested help do.
- The two matching topics on the `help` verb, so the verb spelling reaches the same renderer as the flag spelling.
- Composition of the child's own `doctor` help after the delimiter, for the one routed verb whose name the child also owns.
- The implemented-surface sentence and the verb-scoped flag table on the CLI surface, which today record the gap as outstanding.
- Closing the open question this slice was opened to exit.

## Out of scope

- Requested help for the `help` verb itself, which no current use discriminates and which the topic list deliberately omits until one does.
- Re-enabling the parser's own help flag or help subcommand, which cannot compose the child's answer and cannot choose the stream and status the contract fixes.
- Any change to the reports themselves: the doctor catalog, its machine document, `--list`, `--strict`, and the version output all stay as they are.
- Per-verb man pages, which stay deferred to their own record until a packager needs a named output directory.

## Governed by

- [AGENTS.md](../../../../AGENTS.md)
- [Charter](../../charter.md)
- [ADR-0044](../../../decisions/ADR-0044-audit-wrapper-spellings-against-the-child-inventory.md)
- [ADR-0045](../../../decisions/ADR-0045-compose-doctor-with-the-child-report.md)
- [ADR-0052](../../../decisions/ADR-0052-require-an-explicit-subcommand.md)
- [ADR-0079](../../../decisions/ADR-0079-compose-every-overlapping-surface-with-the-child.md)
- [CLI surface](../../../reference/cli-surface.md)
- [Exit codes](../../../reference/exit-codes.md)
- [Logging and output](../../../reference/logging-and-output.md)
- [Coding conventions](../../../reference/coding-conventions.md)
- [Testing and quality](../../../reference/testing-and-quality.md)

## Acceptance

- When help is requested for `doctor` at either spelling, the wrapper shall write the verb node's help to standard output and shall exit `0`. -> doctor::requested_doctor_help_is_a_result_that_composes_the_child
- When help is requested for `version` at either spelling, the wrapper shall write the verb node's help to standard output and shall exit `0`. -> output::requested_version_help_is_a_result_and_composes_nothing
- Where requested help is written for a verb, the bytes of the flag spelling and of the verb spelling shall be identical.
- When help is requested for `doctor`, the wrapper shall append the child's own help for that verb after a delimiter naming the exact child command.
- When help is requested for `version`, the wrapper shall append nothing from the child.
- If the child cannot be run while requested help is written, then the wrapper shall name the condition in one line and shall still exit `0`.
- When `doctor` or `version` is invoked without requested help, the wrapper shall answer with that verb's report and shall write no help. -> doctor::doctor_without_requested_help_still_reports

## Rabbit holes

- Re-enabling the parser's built-in help to save two declarations; escape: the root spelling composes the child's answer and the built-in cannot, so the opt-in stays per verb.
- Making the topic list total over the verb set while the file is open; escape: only the two spellings the open question names are in scope, and the list carries implemented surfaces only.
- Building a general table of child commands to compose for one overlapping verb; escape: one named case, stated where the composition rule is already read.

## Done when

The four spellings print and exit `0`, the `doctor` half carries the child's help after the delimiter, the CLI surface no longer records the gap, the open question is gone, and `just hooks` is green.

## Revisions

None.
