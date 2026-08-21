# 036 — Agent-keyed sessions

## Goal

Make the word mean what it says. A session is one running coding agent, so it is keyed to that agent's process rather than to the pane the agent was started from, and no session directory outlives the thing it stands for.

## Appetite

2 implementation sessions.

## Core

A session directory belongs to one agent process, and is live exactly while that process is running.

## In scope

- One decision superseding [ADR-0102](../../../decisions/ADR-0102-key-child-state-by-terminal.md): the wrapper execs the child, so the process that becomes the agent is this one, and its identifier and start time — read before the exec — name the directory and answer liveness after it.
- The agent name, scoped by the process namespace that issued the identifier, with the start time in the name because an identifier alone is reused within one boot.
- A witness record naming the agent, at a version that refuses to read the terminal-keyed one, so an existing tree resolves itself under the collection policy rather than by a migration nobody can check.
- One judgment with one question behind it, retiring the naming ladder, the device mapping, the alias ground, and the mount-namespace scope for sessions.
- A sweep of the account's provably dead sessions at launch, because one directory per run would otherwise accumulate between explicit collections. Only the provably dead: a launch is not the place to act on what could not be decided.
- The reader's own row, and `doctor`'s session-scope probes, resolved through the process ancestry — a command is never an agent, so the only session it is in is one it descends from.
- A list report read by scanning: one aligned row per session, the short reason in a column, and no paragraph between the rows.

## Out of scope

- Carrying the child's per-directory state across runs. The transcripts and the peer registry are shared trees the session links to and are unaffected; the rest is what a fresh directory means.
- Removing the launch-path guards this key makes unreachable, which [Q-014](../../open-questions.md#q-014--which-launch-path-guards-survive-a-directory-that-is-always-fresh) owns.
- Classifying a passthrough by what the child would do with it; the wrapper carries no child verb list, and liveness already answers it.

## Governed by

- [AGENTS.md](../../../../AGENTS.md)
- [Charter](../../charter.md)
- [ADR-0051](../../../decisions/ADR-0051-let-every-surface-element-discriminate.md)
- [ADR-0084](../../../decisions/ADR-0084-exec-the-child-instead-of-supervising-it.md)
- [ADR-0089](../../../decisions/ADR-0089-carry-a-child-owned-fact-only-against-an-obligation.md)
- [ADR-0102](../../../decisions/ADR-0102-key-child-state-by-terminal.md)
- [ADR-0107](../../../decisions/ADR-0107-scope-a-terminal-to-its-namespace.md)
- [ADR-0110](../../../decisions/ADR-0110-record-the-terminal-witness-at-launch.md)
- [ADR-0112](../../../decisions/ADR-0112-keep-only-the-session-proven-live.md)
- [Sessions](../../../reference/sessions.md)
- [Doctor](../../../reference/doctor.md)
- [Presentation](../../../reference/presentation.md)
- [Coding conventions](../../../reference/coding-conventions.md)
- [Testing and quality](../../../reference/testing-and-quality.md)

## Acceptance

- When a launch materialises a session directory, the wrapper shall record the process it is about to become and the time that process started, and the directory shall be named after both. -> sessions_gc::a_launch_records_the_witness_naming_its_own_agent
- When a second launch runs, it shall take a session directory of its own rather than the first one's. -> sessions_gc::a_second_launch_is_a_second_session
- When a launch runs, it shall first remove that account's session directories whose agent has exited, and shall leave every other directory standing. -> sessions_gc::a_launch_collects_the_sessions_whose_agents_exited
- When `session list` runs, every session directory shall carry exactly one of `live`, `dead`, or `unknown`, and only one whose agent is running shall be `live`. -> sessions_gc::session_list_tells_the_three_states_apart
- Where a record was written when a session meant a terminal, the wrapper shall not read it, and the directory it named shall be collectable. -> sessions_gc::a_terminal_keyed_session_is_not_read_and_is_collected
- When a report marks a row as the reader's own, it shall mark the agent that reader is running inside and no other. -> sessions_gc::the_reader_row_marks_the_agent_it_runs_inside
- When the work lands, no current document shall describe a session directory as belonging to a terminal.

## Rabbit holes

- Migrating existing terminal-keyed trees; escape: an unreadable record is unaccounted for, and the collection policy already removes what nothing accounts for.
- Reading the child's verb list to tell an agent launch from any other passthrough; escape: no [ADR-0089](../../../decisions/ADR-0089-carry-a-child-owned-fact-only-against-an-obligation.md) obligation covers that list, and a directory whose process has exited is collected without it.
- Rebuilding continuity for the child's per-directory state; escape: the shared trees already carry what survives a run, and inventing a second sharing mechanism is a decision of its own.

## Done when

A launch names its own agent, a second launch is a second session, a launch sweeps what has exited, `session list` scans in one row per session, the terminal-keyed tree resolves itself, and `just hooks` is green.

## Revisions

None.
