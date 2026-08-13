# 020 — Human-first diagnostics

## Goal

Make the report a person reads tell them what is wrong, whether it matters, and what to type next, and stop it from reporting a current child as below the version floor.

## Appetite

2 implementation sessions.

## Core

Every byte the wrapper writes outside `--json` is written for a person, and the version row states a fact rather than a parse failure dressed as one.

## In scope

- One decision making the human format pedagogical by rule: an explanation and a next action where a reader needs one, with identifiers, counters, key-value fields, and exit codes owned by the machine format that already carries them.
- A human title and a plain-language consequence for every catalog entry, so a row leads with what the check protects rather than with its id.
- A rewritten human report and `--list`: passing rows compact, defective and inapplicable rows expanded with a next action, scope headings that say what the scope covers, and the three levels stated in prose.
- The first coloured bytes, over the status word and the scope heading, on the colour decision already resolved and read by nothing.
- Plain prose in every remediation template, replacing the Markdown links and relative document paths a terminal cannot follow.
- A version reading that takes the first version-shaped token from the child's output and ignores the rest, with the unreadable case reported as itself rather than as a below-floor claim.
- A statement of what the version reading still carries from the child, kept to one sentence and recorded where the obligation test can be applied to it.

## Out of scope

- Any change to the `--json` document, its `schema_version`, its field set, or the check ids it publishes; the machine contract is what makes the human rewrite affordable and it does not move.
- Any change to severity, catalog order, exit selection, `--strict`, or which conditions a check reports; this slice changes how a result is said, not what is measured.
- A `--color` flag, a progress indicator, a table crate, or terminal-width layout, each of which a current rule already refuses.
- Rewriting the error diagnostic and the log mirror to the new rule; they are named by it but their renderers are not this slice's work.

## Governed by

- [AGENTS.md](../../../../AGENTS.md)
- [Charter](../../charter.md)
- [ADR-0018](../../../decisions/ADR-0018-one-probe-set-with-stable-check-ids.md)
- [ADR-0024](../../../decisions/ADR-0024-machine-output-is-a-per-verb-flag.md)
- [ADR-0031](../../../decisions/ADR-0031-enforce-the-child-refresh-lock-version-floor.md)
- [ADR-0051](../../../decisions/ADR-0051-let-every-surface-element-discriminate.md)
- [ADR-0081](../../../decisions/ADR-0081-bind-every-human-surface-to-one-presentation-contract.md)
- [ADR-0082](../../../decisions/ADR-0082-colour-a-closed-set-of-named-surfaces.md)
- [ADR-0085](../../../decisions/ADR-0085-carry-the-child-report-level-into-the-verdict.md)
- [ADR-0089](../../../decisions/ADR-0089-carry-a-child-owned-fact-only-against-an-obligation.md)
- [Presentation](../../../reference/presentation.md)
- [Logging and output](../../../reference/logging-and-output.md)
- [Doctor](../../../reference/doctor.md)
- [Research tracking](../../../reference/research-tracking.yaml)
- [Coding conventions](../../../reference/coding-conventions.md)
- [Testing and quality](../../../reference/testing-and-quality.md)

## Acceptance

- When the wrapper reads a version from the child, it shall accept any output containing a version-shaped token and shall use the first one.
- When no version-shaped token is present, the report shall say the version could not be read and shall not claim the child is below the floor.
- When a check passes in human format, its row shall carry the status word, a title, and at most what it observed.
- When a check warns, fails, or does not apply, its row shall carry a consequence and a next action, and shall name its id.
- Where the human format states a level, it shall state it in prose, and shall name an exit status only when the verdict is not a pass.
- Where the human format is written, it shall contain no key-value field, no substitution placeholder, and no Markdown link.
- When the destination is not a capable terminal, the human report bytes shall equal the bytes written with colour stripped.
- When `--json` is requested, the document's shape, field set, check ids, ordering, and schema version shall be unchanged, and no row shall be collapsed.

## Rabbit holes

- Rewriting every renderer the new rule names, when only one verb is in hand; escape: the rule is recorded once and applied to `doctor`, and the next renderer meets it when it is next touched.
- Chasing the child's version line so the parser keeps pace with its spelling; escape: take the first version-shaped token and carry nothing else.
- Reaching for a table, a spinner, or terminal-width alignment to make the report look designed; escape: fixed columns and a fixed wrap, which is what makes the bytes testable.
- Collapsing repeated rows so aggressively that a reader cannot find the id a script matched; escape: a collapsed row names every id it covers.

## Done when

The version row is true against the installed child, every human row explains itself and names a next action, the machine document is byte-identical, the rule is recorded where the next renderer will read it, and `just hooks` is green.

## Revisions

None.
