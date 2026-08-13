# 023 — Human-first verbs

## Goal

Finish what [020](../020-human-first-diagnostics/README.md) started: every byte the wrapper writes outside `--json` is written for a person, so a reader of `account`, `profile`, `config`, an error, or a log line learns what happened and what to do next instead of reading a field dump.

## Appetite

3 implementation sessions.

## Core

No human surface the wrapper owns is still a list of `key: value` lines, and the helpers that make one readable live in one place.

## In scope

- The wrapping, indent, palette, and pluralisation helpers promoted out of the `doctor` renderer into their own module, with their unit tests, so every renderer shares one wrap column and one colour discipline.
- The four account reports rewritten for a person: what the account is, how it authenticates, which profile it runs with and why, what is wrong, and what to type next.
- The profile listing rewritten to say which profile is selected and which layer selected it.
- The empty case of every listing answered in words, since an account list with no accounts currently prints one provenance line about a selection that does not exist, and a profile list with no profiles prints nothing at all.
- The configuration report rewritten to state each resolved value and its layer in words.
- The error diagnostic and the standard-error log mirror rewritten to the same rule, which [ADR-0093](../../../decisions/ADR-0093-write-every-non-machine-surface-for-a-person.md) named and [020](../020-human-first-diagnostics/README.md) deferred.
- The warning wordings the account domain owns rewritten to name the consequence they carry.
- The remaining prose sites a sweep of every wrapper write finds, so the rewrite is the whole surface rather than the verbs alone: the version report, the child-unavailable sentence that prints an error kind at a person, the storage guard's repaired-and-refused events, the logging-unavailable warning that matches neither existing format, and the two terminal prompts, which reach the user through the controlling terminal rather than through the writer.
- One statement of where the rule stops, since a sweep finds surfaces no verb owns: the child's own bytes, the generated completion script and man page, and the developer task runner are each named as outside it rather than left to be inferred.
- The coloured-surface table in [presentation](../../../reference/presentation.md) extended to the surfaces these renderers add, which that page's own rule makes a document edit.
- The published example blocks in [accounts](../../../reference/accounts.md), [configuration](../../../reference/configuration.md), and [the CLI surface](../../../reference/cli-surface.md) taken from the implemented renderers, and the claim that a human report carries the same fields as labelled lines retired.

## Out of scope

- Any change to a `--json` document, its field set, its ordering, or its absent-field rule; the machine contract is what makes the rewrite affordable and it does not move, exactly as in [020](../020-human-first-diagnostics/README.md).
- Any change to what a verb measures, which exit code it selects, or which conditions it reports; this slice changes how a result is said.
- A `--color` flag, a progress indicator, a table crate, or terminal-width layout, each of which a current rule already refuses.
- The `doctor` report, which already meets the rule and is touched only where the promoted helpers move under it.
- The layout `clap` gives `--help` and the man page, which is a generated surface with its own conventions; the authored `about` and `long_about` prose is already written for a person and stays as it is.
- The developer task runner under `xtask`, which no user runs and which writes to its own streams by design.
- The child's own bytes, which pass through unchanged in appearance as in content.

## Governed by

- [AGENTS.md](../../../../AGENTS.md)
- [Charter](../../charter.md)
- [ADR-0024](../../../decisions/ADR-0024-machine-output-is-a-per-verb-flag.md)
- [ADR-0051](../../../decisions/ADR-0051-let-every-surface-element-discriminate.md)
- [ADR-0081](../../../decisions/ADR-0081-bind-every-human-surface-to-one-presentation-contract.md)
- [ADR-0082](../../../decisions/ADR-0082-colour-a-closed-set-of-named-surfaces.md)
- [ADR-0083](../../../decisions/ADR-0083-read-only-the-two-published-colour-variables.md)
- [ADR-0093](../../../decisions/ADR-0093-write-every-non-machine-surface-for-a-person.md)
- [Presentation](../../../reference/presentation.md)
- [Logging and output](../../../reference/logging-and-output.md)
- [Accounts](../../../reference/accounts.md)
- [Configuration](../../../reference/configuration.md)
- [CLI surface](../../../reference/cli-surface.md)
- [Coding conventions](../../../reference/coding-conventions.md)
- [Testing and quality](../../../reference/testing-and-quality.md)

## Acceptance

- Where a human report is written, it shall contain no key-value field, no substitution placeholder, and no Markdown link. -> output::no_human_surface_carries_a_machine_shape
- Where a human report names an identifier, it shall be one the reader supplied or one they must type next.
- When a report states a defect, it shall carry the consequence and the next action, in sentences. -> accounts::an_unbound_account_states_its_consequence_and_the_command_that_fixes_it
- When the destination is not a capable terminal, the human bytes shall equal the coloured bytes with the escapes stripped. -> output::colour_decorates_every_report_without_changing_it
- Where a surface is coloured, it shall appear in the closed table before the renderer emits it.
- When `--json` is requested, every document's shape, field set, and ordering shall be unchanged. -> output::every_machine_document_still_carries_what_the_human_form_dropped
- When a report has nothing to list, it shall say so and name the command that would create the first entry, rather than emitting a bare provenance line or no bytes at all. -> profiles::profile_reports_an_empty_list_without_failing
- Where a document publishes an example of a human report, it shall equal what the renderer produces. -> accounts::the_published_status_example_matches_the_renderer
- Where two renderers wrap prose, they shall wrap at the one constant the presentation contract names. -> ui::prose::tests::prose_wraps_at_a_constant_and_never_splits_a_word
- Where the wrapper writes a byte a person reads, it shall either meet this rule or be named by a document as outside it.

## Rabbit holes

- Redesigning what each verb reports while rewriting how it reports it; escape: the field set is fixed by the machine document, which does not move in this slice.
- Growing a layout engine out of the promoted helpers; escape: a fixed indent and a fixed wrap, which is what makes the bytes testable.
- Colouring whatever looks better; escape: the table is closed and an addition is a document edit first.
- Rewriting the account warnings into paragraphs the machine form cannot carry; escape: a warning stays one value with one sentence.

## Done when

Every wrapper-owned human surface reads as prose, the shared helpers have one home, the coloured set names every surface that emits colour, every published example matches its renderer, the machine documents are byte-identical, and `just hooks` is green.

## Revisions

- 2026-08-13 — A sweep of every site the wrapper writes from found surfaces no verb owns: the version report, a child-unavailable sentence printing an error kind, the storage guard's events, the logging-unavailable warning, and the two terminal prompts. `In scope` gained them and a statement of where the rule stops, and `Acceptance` gained the line that every human byte either meets the rule or is named as outside it.
