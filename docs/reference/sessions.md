# Sessions

The session lifecycle surface: the witness a launch records, the four liveness verdicts, and the `session` verb that reports and collects. Why the scopes exist is in [session isolation](../explanation/session-isolation.md); exact paths, modes, and writers are in [XDG storage](./xdg-storage.md); the decisions are [ADR-0110](../decisions/ADR-0110-record-the-terminal-witness-at-launch.md) and [ADR-0112](../decisions/ADR-0112-keep-only-the-session-proven-live.md).

## The witness record

A session directory's name is a deliberately lossy mapping of its terminal, so the name alone cannot answer whether that terminal still exists. Every launch therefore records the naming inputs — the witness — beside the directory it names, at the path [XDG storage](./xdg-storage.md#artifact-table) lists, mode `0600`, atomically, and only when the recorded bytes would change.

One JSON document, version `1`:

| Field            | Value                                                                      |
| ---------------- | -------------------------------------------------------------------------- |
| `version`        | `1`; any other version is judged unknown rather than parsed                |
| `rung`           | `tty` or `session-leader`, the rung that named the terminal                |
| `device`         | tty rung: the pane's own device path, confirmed against the device node    |
| `sid`, `started` | session-leader rung: the leader's process id and start time in clock ticks |
| `namespace`      | the namespace component the name is unique inside                          |
| `boot`           | the kernel's boot identifier at recording time; absent when unreadable     |

The tty rung reads the pane from `tty_nr`, field 7 of `/proc/self/stat`: the controlling terminal's device number, which no stream redirection changes. The number is mapped onto the path the device is published at — `/dev/pts/<index>` under the eight majors devpts registers, `/dev/tty<n>` for a virtual console — and the candidate is then confirmed by reading that node's own device number back. A candidate that does not confirm names nothing, so the rung falls through to the session leader rather than keying one pane's state to another's directory. Opening `/dev/tty` and asking its name back is deliberately not how this works: that node is the alias every process shares, so it names no pane and exists whether or not a terminal does.

A launch that cannot record its witness — an unwritable parent, a device path outside UTF-8 — warns and launches anyway: recording is additive, and the cost is that the session is judged unknown, which is collectable, until a launch that can record one.

## Verdicts

Liveness re-asks the naming question against the record. The session tree is the wrapper's own, so a directory in it is either a session this run can account for or it is garbage: `live` keeps a directory and every other verdict is collectable ([ADR-0112](../decisions/ADR-0112-keep-only-the-session-proven-live.md)).

| Record                                                                                      | Verdict    |
| ------------------------------------------------------------------------------------------- | ---------- |
| Missing, unreadable, another version, or filed under a namespace directory it does not name | `unknown`  |
| Namespace component is not this run's, or this run cannot derive one                        | `unknown`  |
| tty rung, in scope, recording `/dev/tty`, which names no pane                               | `orphaned` |
| tty rung, in scope, device path exists                                                      | `live`     |
| tty rung, in scope, device path absent                                                      | `dead`     |
| tty rung, in scope, whether the device path exists could not be observed                    | `unknown`  |
| session-leader rung, in scope, recorded boot is not this one                                | `dead`     |
| session-leader rung, in scope, same boot, process id live with the recorded start time      | `live`     |
| session-leader rung, in scope, same boot, process absent or started at another time         | `dead`     |
| session-leader rung, in scope, either boot or the start time unreadable                     | `unknown`  |

A verdict is the projection of the ground it stands on, and both are reported: `unrecorded`, `unplaced`, `foreign`, `alias`, `device-present`, `device-absent`, `device-unobservable`, `leader-running`, `leader-gone`, `leader-foreign-boot`, `leader-unreadable`, in the row order of the table above. The ground is what the human report turns into a sentence and what the `--json` document names, so neither has to re-derive why a directory was kept or what collecting it costs.

Four words for one policy, because they differ in what a reader loses rather than in what the verb does. `dead` belonged to a terminal that has closed. `orphaned` was never owned by one terminal at all: it is the residue of a version that asked the alias for its own name, so it gave every pane of one namespace a single directory, and no terminal number this version maps can ever land on that name again — nothing will claim it, which is exactly why nothing should keep it. `unknown` could not be accounted for: no readable record, a namespace this kernel cannot see, or an observation that failed. None of the three is a gap the wrapper waits out, because a directory nothing accounts for and nothing will ever claim is not an open question but an accumulation.

The tty rung ignores boot deliberately: a reopened `/dev/pts/0` after reboot is the same slot, and slot reuse is intended, so the slot's history survives the reboot. A directory that predates its witness self-heals rather than being collected, provided a launch reaches it first.

One state stops the verb instead of feeding it. A run that cannot name the namespace its own directories are scoped by has placed no record at all, so every row would read `unplaced` and the collector would empty the tree on the strength of its own blindness. `session clean` refuses there, `Unavailable`, before any side effect; `session list` reports the rows and says so instead of offering the verb.

## Commands

| Command                          | Regime                                             | Effect                                                                    |
| -------------------------------- | -------------------------------------------------- | ------------------------------------------------------------------------- |
| `session list [--json]`          | [Inspection](./exit-codes.md#exit-regimes-by-verb) | Report every session directory of every account, judged                   |
| `session clean [--yes] [--json]` | [Operation](./exit-codes.md#exit-regimes-by-verb)  | Preview the collectable set, confirm, remove it; `--yes` skips the prompt |

Bare `session` is malformed, like bare `account` ([ADR-0052](../decisions/ADR-0052-require-an-explicit-subcommand.md)). Requested help composes nothing, because the child owns no verb of this name.

`session list` orders findings by account, namespace, then terminal, so two surveys of one unchanged tree report identically. The `--json` document is `{"sessions": [...]}` where each row carries `account`, `namespace`, `terminal`, `verdict`, `ground`, `current`, `path`, plus `rung` only when a witness was read, and `names` only when that witness names a terminal — so a legacy record naming `/dev/tty` carries a `rung` and no `names`, because the alias names no pane. `current` marks every session directory this run's own terminal owns, which is more than one row when that terminal has launched under more than one account, and is `false` for every row when the run cannot name its own terminal.

The human report is one row per session: the verdict as a bracketed word, the terminal and account it is about, and one sentence giving the ground and whether the session is kept, collectable, or the state that refuses the verb. A collectable row names its directory as well, on a line of its own, because a row whose record names no terminal has nothing else telling it apart. The report says in words that a session directory outlives the child that made it, so a `live` row is not read as an agent still running in that pane.

`session clean` confirms on the controlling terminal, never on standard input, and the preview is part of the question: the prompt lists every directory it would delete before asking, grouped by verdict, because the three groups cost a reader three different things. One question covers them all, and `--yes` skips it. Declining removes nothing and exits `0`, an outcome rather than an error. With no controlling terminal and no `--yes` it refuses `Unavailable` before any side effect, as it does when the run cannot place itself. The `--json` document is `{"removed": [...], "pruned_namespaces": n}`, with `"declined": true` added when the prompt was refused.

Removal runs per account under that account's [write lock](./xdg-storage.md#lock-scopes), so it never interleaves with an `account remove` destroying the same scope. Each directory is re-validated by the guard before deletion — never through a symbolic link — and its witness is removed after it, so a crash between the two leaves an orphan record rather than an undecidable directory. A namespace directory is removed only once nothing but orphan records is left inside it, and an orphan record is swept with it. Nothing collects automatically: no launch, no schedule, only this verb.

## What clean never touches

Every session whose terminal this run can still see, the account's `config/` tree and credential, composed settings entries — whose growth [XDG storage](./xdg-storage.md#composed-settings-entries) already judged too slow to earn a policy — and the peer registry, whose foreign-boot scopes may be another kernel's live boots.

One cost is named rather than designed around. On a state tree shared with another kernel — a container's bind mount, a virtual machine's filesystem share — that side's session directories are `unknown` here and therefore collectable here, including ones live on that side. The tree is the wrapper's to account for, and a directory this run cannot account for is garbage by the same rule wherever it came from ([ADR-0112](../decisions/ADR-0112-keep-only-the-session-proven-live.md)).
