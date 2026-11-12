# 042 — Session list describes the child

## Goal

Make `session list` show each session's child session id and session kind beside its name, status, and working directory, in the human table and in the `--json` document.

## Appetite

1 implementation session.

## Core

A row carries the child's session id and kind only from a registration that the witness join proved, and loses them on every failure that costs the row its name.

## In scope

- Read the registration's session id and kind under [ADR-0132](../../../decisions/ADR-0132-identify-a-reported-session-by-the-children-id-and-kind.md).
- Carry both facts in the finding, and clear them wherever the name is cleared.
- Add the optional `session_id` and `kind` fields to every document row.
- Give the human table the columns `status`, `session`, `kind`, `id`, `account`, `why`, and `directory`.

## Out of scope

- Filtering `session list` by session id. A caller that holds the id filters the document.
- The child's name provenance, version, and start time, which ADR-0132 rejects.
- Any change to a verdict or to what `session clean` takes.
- The child's messaging socket and token, which no report reads or writes.
- Slice 034. The operator placed this slice ahead of it on 2026-11-12, and 034 starts after this slice closes.

## Governed by

- [Agent guidelines](../../../../AGENTS.md)
- [Planning charter](../../charter.md)
- [ADR-0051](../../../decisions/ADR-0051-let-every-surface-element-discriminate.md)
- [ADR-0089](../../../decisions/ADR-0089-carry-a-child-owned-fact-only-against-an-obligation.md)
- [ADR-0093](../../../decisions/ADR-0093-write-every-non-machine-surface-for-a-person.md)
- [ADR-0114](../../../decisions/ADR-0114-name-a-reported-session-as-the-child-does.md)
- [ADR-0124](../../../decisions/ADR-0124-describe-a-reported-session-with-the-children-facts.md)
- [ADR-0132](../../../decisions/ADR-0132-identify-a-reported-session-by-the-children-id-and-kind.md)
- [Sessions](../../../reference/sessions.md)
- [Presentation](../../../reference/presentation.md)
- [Logging and output](../../../reference/logging-and-output.md)
- [Child facts](../../../reference/child-facts.yaml)
- [Development workflow](../../../guides/development-workflow.md)

## Acceptance

- Where a proven registration carries a session id and a kind, the document row shall carry them as `session_id` and `kind`.
- Where a registration carries no valid session id or kind, the document row shall omit that field rather than write `null`.
- If two sessions claim one registration, then neither row shall carry a session id or a kind.
- The human table shall show the kind, the first eight characters of the session id, and the working directory for each described row.
- The report shall not carry the child's messaging socket path or token in either form.

## Rabbit holes

- Two rows whose short ids match; escape: the document carries the whole id, and the table is read by a person.
- A table wider than the terminal; escape: the presentation rule forbids reading the width, so put the directory last and stop.
- Carrying the child's vocabulary of kinds; escape: carry the word verbatim, as the status word is carried.

## Done when

Every acceptance statement is implemented, its named test resolves, the canon describes the result, and `just hooks` is green.

## Revisions

None.
