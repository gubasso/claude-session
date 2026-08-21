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

The name comes from the child's own registration under the [shared peer registry](../explanation/session-isolation.md), which every session's `sessions` name links to ([ADR-0108](../decisions/ADR-0108-share-the-child-peer-registry-across-sessions.md)). A report reads the registry of this run's own scope, takes the `name` of each registration, and keeps it only for the session whose witness the registration's process identifier and `procStart` both match. That pair is what makes the name safe: a process identifier is reused within one boot, so without the start time a live agent's registration would lend its name to the exited session that ran under the same number.

Nothing else in that record is read. The working directory, the child's session identifier, and its busy state are the child's own, and no wrapper obligation reaches them ([ADR-0089](../decisions/ADR-0089-carry-a-child-owned-fact-only-against-an-obligation.md)). The record is undocumented, so its freshness is [tracked](./research-tracking.yaml).

Every failure costs a row its name and nothing else: a scope this run cannot derive, a registry that is not there, a record that does not parse, a name that does not verify, and a name holding a control character all leave the row named by its directory. A name is never truncated to fit a column, and a registration never changes a verdict — liveness is the witness's question, and a registry the wrapper does not write cannot answer it.

## Commands

| Command                          | Regime                                             | Effect                                                                    |
| -------------------------------- | -------------------------------------------------- | ------------------------------------------------------------------------- |
| `session list [--json]`          | [Inspection](./exit-codes.md#exit-regimes-by-verb) | Report every session directory of every account, judged                   |
| `session clean [--yes] [--json]` | [Operation](./exit-codes.md#exit-regimes-by-verb)  | Preview the collectable set, confirm, remove it; `--yes` skips the prompt |

Bare `session` is malformed, like bare `account` ([ADR-0052](../decisions/ADR-0052-require-an-explicit-subcommand.md)). Requested help composes nothing, because the child owns no verb of this name.

`session list` orders findings by account, namespace, then session, so two surveys of one unchanged tree report identically. The `--json` document is `{"sessions": [...]}` where each row carries `account`, `namespace`, `session`, `verdict`, `ground`, `current`, `path`, plus `pid` only when a record this version reads was read and `name` only when a registration verified against that record.

`current` marks the agent the reading command is running inside. A command is never an agent and never has a session directory of its own, so the only session it can be in is one it descends from; the ancestry says which, and the process identifier and start time together are what confirm it. Every row is `false` when the command is not running under an agent, which is the ordinary case from a shell.

The human report is one table — a `status` column carrying the verdict as a bracketed word, then `session`, `account`, and `why` — with a column-name row, a rule under it, and nothing between the rows and the summary line beneath them. The `session` column carries the name above, and the directory only where there is none; the directory is on every row of the document, which is where a caller wanting it was always meant to look. A list is read by scanning, so the long form of any reason lives on this page rather than in the report, and the columns are computed from the rows alone rather than from the terminal ([presentation](./presentation.md#tables-progress-and-prompts)).

`session clean` confirms on the controlling terminal, never on standard input, and the preview is part of the question: the prompt lists every directory it would delete before asking, grouped by verdict, because the two groups cost a reader different things. One question covers them all, and `--yes` skips it. Declining removes nothing and exits `0`, an outcome rather than an error. With no controlling terminal and no `--yes` it refuses `Unavailable` before any side effect, as it does when the run cannot place itself. The `--json` document is `{"removed": [...], "pruned_namespaces": n}`, with `"declined": true` added when the prompt was refused.

Removal runs per account under that account's [write lock](./xdg-storage.md#lock-scopes), so it never interleaves with an `account remove` destroying the same scope. Each directory is re-validated by the guard before deletion — never through a symbolic link — and its witness is removed after it, so a crash between the two leaves an orphan record rather than a directory nothing accounts for. A namespace directory is removed only once nothing but orphan records is left inside it, and an orphan record is swept with it. The verb is the only thing that collects an undecidable directory; a launch collects the provably dead of its own account and nothing else.

## What clean never touches

Every session whose agent is running, the account's `config/` tree and credential, composed settings entries — whose growth [XDG storage](./xdg-storage.md#composed-settings-entries) already judged too slow to earn a policy — and the peer registry, whose foreign-boot scopes may be another kernel's live boots.

One cost is named rather than designed around. On a state tree shared with another kernel — a container's bind mount, a virtual machine's filesystem share — that side's session directories are `unknown` here and therefore collectable here, including ones live on that side. The tree is the wrapper's to account for, and a directory this run cannot account for is garbage by the same rule wherever it came from ([ADR-0112](../decisions/ADR-0112-keep-only-the-session-proven-live.md)).
