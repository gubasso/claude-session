# 039 — Cross-container session lookup

## Goal

Make `session list` name a session another container launched and let a caller ask for one session by the name its row carries.

## Appetite

2 implementation sessions.

## Core

A report names a session from the registry that session's own launch pointed at, and never borrows a name that is not that session's.

## In scope

- Resolve each row's registration through its own declared registry link.
- Report whether that registry is this run's reachable scope.
- Filter `session list` by an exact child name or session directory.
- Correct the human wording for a namespace this run cannot see.
- Carry the child's working directory and status word in document rows.

## Out of scope

- Changing what `session clean` takes; [ADR-0112](../../../decisions/ADR-0112-keep-only-the-session-proven-live.md) settled that policy.
- Cross-container messaging or reproducing the child's socket paths.
- Prefix, glob, fuzzy, or case-insensitive matching.

## Governed by

- [Agent guidelines](../../../../AGENTS.md)
- [Planning charter](../../charter.md)
- [ADR-0089](../../../decisions/ADR-0089-carry-a-child-owned-fact-only-against-an-obligation.md)
- [ADR-0108](../../../decisions/ADR-0108-share-the-child-peer-registry-across-sessions.md)
- [ADR-0112](../../../decisions/ADR-0112-keep-only-the-session-proven-live.md)
- [ADR-0114](../../../decisions/ADR-0114-name-a-reported-session-as-the-child-does.md)
- [ADR-0123](../../../decisions/ADR-0123-read-a-sessions-own-peer-registry.md)
- [ADR-0124](../../../decisions/ADR-0124-describe-a-reported-session-with-the-children-facts.md)
- [Sessions](../../../reference/sessions.md)
- [Command surface](../../../reference/cli-surface.md)
- [Presentation](../../../reference/presentation.md)
- [Exit codes](../../../reference/exit-codes.md)
- [Development workflow](../../../guides/development-workflow.md)

## Acceptance

- Where a session's own `sessions` link names a registry, `session list` shall name that session from the registrations in that registry. -> sessions_gc::a_session_is_named_from_the_registry_its_own_link_names
- Where a session's registry is not this run's own scope, the report shall name that session and shall report it as not reachable. -> sessions_gc::a_session_of_another_scope_is_named_and_marked_unreachable
- If a session's `sessions` link does not name a registry under the peer root, then the report shall name that session by its directory. -> sessions_gc::a_link_outside_the_peer_root_names_nothing
- Where two registrations of one registry answer to a session's process identifier and start time, the report shall name that session by its directory. -> sessions_gc::an_ambiguous_registration_names_nothing
- While a session of another scope is reported, `session clean` shall take exactly the directories it took before. -> sessions_gc::naming_a_session_of_another_scope_does_not_change_what_clean_takes
- When `session list` is given a name, it shall report only the rows whose child name or session directory is exactly that name. -> sessions_gc::a_named_list_reports_only_the_rows_that_answer_to_it
- If a name matches no session, then `session list` shall report an empty result and exit zero. -> sessions_gc::a_name_matching_nothing_is_an_empty_report_at_exit_zero
- The report shall not describe a session of a namespace this run cannot see as another machine's. -> sessions_gc::a_row_this_run_cannot_place_is_not_called_another_machines
- Where a registration carries the child's working directory and status word, the document row shall carry them and the human table shall keep its four columns. -> sessions_gc::the_document_carries_the_children_directory_and_status_word

## Rabbit holes

- Making a foreign row uncollectable; escape: it supersedes ADR-0112, so route it to open questions and leave the destructive verb alone.
- Reproducing the child's socket path so a peer can be messaged; escape: the sockets are not on a shared path, so report reachability and stop.
- Widening the match rule to a prefix or a glob; escape: a machine caller filters the document, and a narrow rule widens later.

## Done when

Every acceptance statement is implemented, its named test resolves, the canon describes the result, and `just hooks` is green.

## Revisions

None.
