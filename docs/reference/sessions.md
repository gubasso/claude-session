# Sessions

The session lifecycle surface: what a session is, the record a launch writes, the three liveness verdicts, and the `session` verb that reports and collects. Why the scopes exist is in [session isolation](../explanation/session-isolation.md); exact paths, modes, and writers are in [XDG storage](./xdg-storage.md); the decisions are [ADR-0112](../decisions/ADR-0112-keep-only-the-session-proven-live.md) and [ADR-0113](../decisions/ADR-0113-key-a-session-to-its-running-agent.md).

## What a session is

One running coding agent, and nothing else the wrapper or the child is asked to do. A launch that starts an agent gets a session directory; the directory belongs to that agent alone and lasts exactly as long as it runs.

The wrapper execs the child ([ADR-0084](../decisions/ADR-0084-exec-the-child-instead-of-supervising-it.md)), so the process that becomes the agent is the wrapper's own. Its identifier and start time are read before the exec and survive it unchanged, which is what lets a directory named before the exec answer for the agent after it. The name is `agent-<pid>-<started>`, the start time in hexadecimal, under the namespace directory that scopes the process identifier.

The start time is part of the name rather than a field beside it: a process identifier is reused within one boot, and without the start time a later agent would walk into an earlier one's directory.

A launch removes that account's session directories whose agent has exited before creating its own. Only the provably dead: a launch is not the place to act on what could not be decided, and an undecidable directory is exactly the one a person should see named before it goes.

## The record

A directory's name spells its agent, but a name is not evidence — a directory can be moved, and a start time counts ticks from a boot the name never states. Every launch therefore records the naming inputs beside the directory it names, at the path [XDG storage](./xdg-storage.md#artifact-table) lists, mode `0600`, atomically.

One JSON document, version `2`:

| Field       | Value                                                           |
| ----------- | --------------------------------------------------------------- |
| `version`   | `2`; any other version is not read                              |
| `pid`       | the process the agent runs as                                   |
| `started`   | that process's start time, in clock ticks since `boot`          |
| `namespace` | the namespace component the process identifier is unique inside |
| `boot`      | the kernel boot the start time counts from                      |

`boot` is not optional. A start time that cannot say which boot it counts from cannot be judged at all, so a launch that cannot read one records nothing rather than writing a record no run can use.

Version `1` keyed a session to a terminal. It is not read and not translated: it witnesses a question this version no longer asks, so the directory it named is unaccounted for and the collection policy removes it ([ADR-0113](../decisions/ADR-0113-key-a-session-to-its-running-agent.md)).

A launch that cannot record its witness warns and launches anyway: recording is additive, and the cost is that the session is unaccounted for, and therefore collectable, rather than an exec this run refuses.

## Verdicts

Liveness asks one question of the record: is that agent still running. The tree is the wrapper's own, so `live` keeps a directory and every other verdict is collectable ([ADR-0112](../decisions/ADR-0112-keep-only-the-session-proven-live.md)).

| Record                                                                            | Verdict   |
| --------------------------------------------------------------------------------- | --------- |
| Missing, unreadable, another version, or filed under a namespace it does not name | `unknown` |
| Namespace component is not this run's                                             | `unknown` |
| In scope, recorded boot is not this one                                           | `dead`    |
| In scope, same boot, process running with the recorded start time                 | `live`    |
| In scope, same boot, process absent or started at another time                    | `dead`    |
| In scope, same boot, the process's start time could not be read                   | `unknown` |
| This run can name neither its namespace nor its boot                              | `unknown` |

A verdict is the projection of the ground it stands on, and both are reported: `unrecorded`, `unplaced`, `foreign`, `foreign-boot`, `running`, `gone`, `unreadable`, in the row order of the table above. The ground is what the human report turns into a phrase and what the `--json` document names.

A boot that has ended answers the question without looking at any process, because none outlives its kernel. A reissued process identifier is as gone as an absent one, and the start time is what tells the two apart.

One state stops the verb instead of feeding it. A run that can name neither the namespace its directories are scoped by nor its own boot has placed no record, so every row would read `unplaced` and the collector would empty the tree on the strength of its own blindness. `session clean` refuses there, `Unavailable`, before any side effect; `session list` reports the rows and says so instead of offering the verb.

## The name a row carries

A directory is named `agent-<pid>-<started>`, which is the wrapper's own identifier and not a name anybody has seen. The name a person knows a session by is the child's: the one in a status line, the one its rename sets. A report therefore names each session as its reader does, and falls back to the directory only where there is no such name ([ADR-0114](../decisions/ADR-0114-name-a-reported-session-as-the-child-does.md)).

The name comes from the child's own registration under the [shared peer registry](../explanation/session-isolation.md), which every session's `sessions` name links to ([ADR-0108](../decisions/ADR-0108-share-the-child-peer-registry-across-sessions.md)). A report reads the registry each row's own link names. It re-anchors the target's final `peers/<boot>/<namespace>` components under this run's peer root and never opens the foreign target ([ADR-0123](../decisions/ADR-0123-read-a-sessions-own-peer-registry.md)). It keeps a registration only when exactly one has a process identifier and `procStart` matching the witness.

The report also carries the registration's working directory and child status word under `describe-the-subject` ([ADR-0124](../decisions/ADR-0124-describe-a-reported-session-with-the-children-facts.md)). Both are optional, the status vocabulary remains the child's, and neither affects a verdict. Every other field remains unread. The undocumented record's freshness is [tracked](./research-tracking.yaml).

Every failure costs a row the child's whole description of it — its `name`, its `working_directory`, and its `claude_status` — and nothing else. The wrapper's own fields stay: the identity columns, `verdict`, `ground`, `pid`, `current`, `reachable`, and `path` are unaffected. A missing link, a real `sessions` directory, a target outside the registry shape, an absent registry, an invalid record or name, two registrations answering to one pair, and one registration two sessions of a registry both answer to all leave the row named by its directory and described by nothing. The join is guarded on both sides because the child keys a registration by the process identifier alone: two process namespaces sharing one mount namespace share one registry and one filename, so a pair issued in both leaves one record where two sessions claim it. A name is never truncated, and a registration never changes a verdict.

Every document row carries `reachable`, true exactly when its resolved registry is this run's own peer scope. It promises nothing about a socket or messaging: the child binds sockets under a path containers do not share, and the wrapper messages nobody. This is not the `foreign` ground: that ground compares process namespaces, while reachability compares the mount namespace and boot embodied by registry paths.

## Commands

| Command                          | Regime                                             | Effect                                                                    |
| -------------------------------- | -------------------------------------------------- | ------------------------------------------------------------------------- |
| `session list [<name>] [--json]` | [Inspection](./exit-codes.md#exit-regimes-by-verb) | Report every session, or exact matches by child name or directory         |
| `session clean [--yes] [--json]` | [Operation](./exit-codes.md#exit-regimes-by-verb)  | Preview the collectable set, confirm, remove it; `--yes` skips the prompt |

Bare `session` is malformed, like bare `account` ([ADR-0052](../decisions/ADR-0052-require-an-explicit-subcommand.md)). Requested help composes nothing, because the child owns no verb of this name.

`session list` orders findings by account, namespace, then session. Its optional name matches exactly and case-sensitively against either the child name or session directory; several rows may match, and none is an empty report at exit `0`. The `--json` document is `{"sessions": [...]}` where each row carries `account`, `namespace`, `session`, `verdict`, `ground`, `current`, `reachable`, and `path`; `pid` and `name` remain conditional, and `working_directory` and `claude_status` are omitted when absent. The collector's removed rows use this same shape. The human form of a named listing does not count what `session clean` would take, because that verb surveys the whole tree and takes no filter; it points at the unfiltered listing instead.

`current` marks the agent the reading command is running inside. A command is never an agent and never has a session directory of its own, so the only session it can be in is one it descends from; the ancestry says which, and the process identifier and start time together are what confirm it. Every row is `false` when the command is not running under an agent, which is the ordinary case from a shell.

The human report is one table — a `status` column carrying the verdict as a bracketed word, then `session`, `account`, and `why` — with a column-name row, a rule under it, and nothing between the rows and the summary line beneath them. The `session` column carries the name above, and the directory only where there is none; the directory is on every row of the document, which is where a caller wanting it was always meant to look. A list is read by scanning, so the long form of any reason lives on this page rather than in the report, and the columns are computed from the rows alone rather than from the terminal ([presentation](./presentation.md#tables-progress-and-prompts)).

`session clean` confirms on the controlling terminal, never on standard input, and the preview is part of the question: the prompt lists every directory it would delete before asking, grouped by verdict, because the two groups cost a reader different things. One question covers them all, and `--yes` skips it. Declining removes nothing and exits `0`, an outcome rather than an error. With no controlling terminal and no `--yes` it refuses `Unavailable` before any side effect, as it does when the run cannot place itself. The `--json` document is `{"removed": [...], "pruned_namespaces": n}`, with `"declined": true` added when the prompt was refused.

Removal runs per account under that account's [write lock](./xdg-storage.md#lock-scopes), so it never interleaves with an `account remove` destroying the same scope. Each directory is re-validated by the guard before deletion — never through a symbolic link — and its witness is removed after it, so a crash between the two leaves an orphan record rather than a directory nothing accounts for. A namespace directory is removed only once nothing but orphan records is left inside it, and an orphan record is swept with it. The verb is the only thing that collects an undecidable directory; a launch collects the provably dead of its own account and nothing else.

## What clean never touches

Every session whose agent is running, the account's `config/` tree and credential, composed settings entries — whose growth [XDG storage](./xdg-storage.md#composed-settings-entries) already judged too slow to earn a policy — and the peer registry, whose foreign-boot scopes may be another kernel's live boots.

One cost is named rather than designed around. On a state tree shared across an unseen namespace or kernel, that side's directories are `unknown` here and therefore collectable, including live ones. Such a row can now be named while `session clean` still treats it as garbage under [ADR-0112](../decisions/ADR-0112-keep-only-the-session-proven-live.md); [Q-019](../plan/open-questions.md#q-019--should-a-session-proven-to-exist-in-another-registry-be-uncollectable) asks whether that should change.
