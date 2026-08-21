# 037 — Session report by name

## Goal

Let a person recognise their own sessions in the report about them. A row names the session the way its reader already knows it — the name in their status line, the name `/rename` sets — and the rows are laid out as one table read by scanning.

## Appetite

1 implementation session.

## Core

A session's row names it as its reader does, and never borrows a name that is not that session's.

## In scope

- One decision permitting the wrapper to read the child's own name for a running agent, against an obligation [ADR-0089](../../../decisions/ADR-0089-carry-a-child-owned-fact-only-against-an-obligation.md)'s three do not cover.
- The join that makes the name safe: the registration's process identifier and start time, compared with the witness this wrapper wrote, so a reissued identifier cannot lend its name to another agent's directory.
- The name in the `--json` document, present only when a registration verified against that witness.
- A table shape in the [presentation](../../../reference/presentation.md) contract, which today says the wrapper renders none: a header row, a rule, columns padded to a constant computed from the rows, and no colour beyond the status token and the heading.
- Registry and freshness entries for the record fields the report reads, because the record is the child's and is undocumented.

## Out of scope

- Reading anything else the registration holds. The working directory, the session identifier, and the busy state are the child's, and no obligation reaches them.
- Naming a session in any other report. `doctor` answers whether this run can launch, and a name changes no answer it gives.
- Reaching a peer over the socket the registration names, which is the child's own mechanism and not a wrapper surface.
- Letting a registration decide liveness. The witness answers that, and a registry the wrapper does not write may not overrule it.

## Governed by

- [AGENTS.md](../../../../AGENTS.md)
- [Charter](../../charter.md)
- [ADR-0051](../../../decisions/ADR-0051-let-every-surface-element-discriminate.md)
- [ADR-0082](../../../decisions/ADR-0082-colour-a-closed-set-of-named-surfaces.md)
- [ADR-0089](../../../decisions/ADR-0089-carry-a-child-owned-fact-only-against-an-obligation.md)
- [ADR-0093](../../../decisions/ADR-0093-write-every-non-machine-surface-for-a-person.md)
- [ADR-0108](../../../decisions/ADR-0108-share-the-child-peer-registry-across-sessions.md)
- [ADR-0112](../../../decisions/ADR-0112-keep-only-the-session-proven-live.md)
- [ADR-0113](../../../decisions/ADR-0113-key-a-session-to-its-running-agent.md)
- [Sessions](../../../reference/sessions.md)
- [Presentation](../../../reference/presentation.md)
- [Child facts](../../../reference/child-facts.yaml)
- [Coding conventions](../../../reference/coding-conventions.md)
- [Testing and quality](../../../reference/testing-and-quality.md)

## Acceptance

- Where the child registered a name for a session's agent, `session list` shall name that session by it in both forms. -> sessions_gc::a_session_is_named_as_the_child_registered_it
- Where a registration names a process identifier whose start time is not that session's, the report shall not carry its name. -> sessions_gc::a_registration_of_another_start_time_names_nothing
- Where no registration names a session's agent, the report shall name that session by its directory. -> sessions_gc::an_unregistered_session_is_named_by_its_directory
- The human report shall lay its rows out as one table whose columns are computed from the rows alone. -> sessions_gc::the_report_lays_its_rows_out_as_one_table
- When a registration carries a name holding a control character, the report shall refuse it rather than render it. -> sessions_gc::a_registered_name_holding_a_control_character_is_refused

## Rabbit holes

- Carrying the rest of the registration because it is already parsed; escape: an unregistered field is a carry with no obligation, and [ADR-0089](../../../decisions/ADR-0089-carry-a-child-owned-fact-only-against-an-obligation.md) removes it rather than recording it.
- A general table engine for every future report; escape: one verb needs one table, and the shape lives beside the wrap constant every renderer already imports.
- Reading the registry to tell live from dead; escape: the witness already answers it, and the registry is a convenience the launch degrades without.

## Done when

A row names its session as the child does, an unverifiable registration names nothing, the rows read as one table, the carried fields are registered and tracked, and `just hooks` is green.

## Revisions

None.
