# Sessions

The session lifecycle surface: the witness a launch records, the three liveness verdicts, and the `session` verb that reports and collects. Why the scopes exist is in [session isolation](../explanation/session-isolation.md); exact paths, modes, and writers are in [XDG storage](./xdg-storage.md); the decisions are [ADR-0110](../decisions/ADR-0110-record-the-terminal-witness-at-launch.md) and [ADR-0111](../decisions/ADR-0111-collect-only-the-provably-dead-session.md).

## The witness record

A session directory's name is a deliberately lossy mapping of its terminal, so the name alone cannot answer whether that terminal still exists. Every launch therefore records the naming inputs — the witness — beside the directory it names, at the path [XDG storage](./xdg-storage.md#artifact-table) lists, mode `0600`, atomically, and only when the recorded bytes would change.

One JSON document, version `1`:

| Field            | Value                                                                      |
| ---------------- | -------------------------------------------------------------------------- |
| `version`        | `1`; any other version is judged unknown rather than parsed                |
| `rung`           | `tty` or `session-leader`, the rung that named the terminal                |
| `device`         | tty rung: the controlling terminal's device path                           |
| `sid`, `started` | session-leader rung: the leader's process id and start time in clock ticks |
| `namespace`      | the namespace component the name is unique inside                          |
| `boot`           | the kernel's boot identifier at recording time; absent when unreadable     |

A launch that cannot record its witness — an unwritable parent, a device path outside UTF-8 — warns and launches anyway: recording is additive, and the cost is that the session is judged unknown until a launch that can record one.

## Verdicts

Liveness re-asks the naming question against the record. Three verdicts, and only `dead` is ever collectable ([ADR-0111](../decisions/ADR-0111-collect-only-the-provably-dead-session.md)):

| Record                                                                                      | Verdict   |
| ------------------------------------------------------------------------------------------- | --------- |
| Missing, unreadable, another version, or filed under a namespace directory it does not name | `unknown` |
| Namespace component is not this run's, or this run cannot derive one                        | `unknown` |
| tty rung, in scope, device path exists                                                      | `live`    |
| tty rung, in scope, device path absent                                                      | `dead`    |
| session-leader rung, in scope, recorded boot is not this one                                | `dead`    |
| session-leader rung, in scope, same boot, process id live with the recorded start time      | `live`    |
| session-leader rung, in scope, same boot, process absent or started at another time         | `dead`    |
| session-leader rung, in scope, either boot or the start time unreadable                     | `unknown` |

Two verdicts are deliberate boundaries rather than gaps. The tty rung ignores boot: a reopened `/dev/pts/0` after reboot is the same slot, and slot reuse is intended, so the slot's history survives the reboot. And `unknown` is never collected, because on a shared state tree — a container's bind mount, a virtual machine's filesystem share — another kernel's sessions are visible without being decidable, and absence of evidence there is not death. A directory that predates its witness self-heals: its next launch records one.

## Commands

| Command                          | Regime                                             | Effect                                                             |
| -------------------------------- | -------------------------------------------------- | ------------------------------------------------------------------ |
| `session list [--json]`          | [Inspection](./exit-codes.md#exit-regimes-by-verb) | Report every session directory of every account, judged            |
| `session clean [--yes] [--json]` | [Operation](./exit-codes.md#exit-regimes-by-verb)  | Preview the dead set, confirm, remove it; `--yes` skips the prompt |

Bare `session` is malformed, like bare `account` ([ADR-0052](../decisions/ADR-0052-require-an-explicit-subcommand.md)). Requested help composes nothing, because the child owns no verb of this name.

`session list` orders findings by account, namespace, then terminal, so two surveys of one unchanged tree report identically. The `--json` document is `{"sessions": [...]}` where each row carries `account`, `namespace`, `terminal`, `verdict`, `path`, and — only when a witness was read — `rung`.

`session clean` confirms on the controlling terminal, never on standard input, and the preview is part of the question: the prompt lists every directory it would delete before asking. Declining removes nothing and exits `0`, an outcome rather than an error. With no controlling terminal and no `--yes` it refuses `Unavailable` before any side effect. The `--json` document is `{"removed": [...], "pruned_namespaces": n}`, with `"declined": true` added when the prompt was refused.

Removal runs per account under that account's [write lock](./xdg-storage.md#lock-scopes), so it never interleaves with an `account remove` destroying the same scope. Each directory is re-validated by the guard before deletion — never through a symbolic link — and its witness is removed after it, so a crash between the two leaves an orphan record rather than an undecidable directory. A namespace directory is removed only once nothing but orphan records is left inside it, and an orphan record is swept with it. Nothing collects automatically: no launch, no schedule, only this verb.

## What clean never touches

Live and unknown sessions, the account's `config/` tree and credential, composed settings entries — whose growth [XDG storage](./xdg-storage.md#composed-settings-entries) already judged too slow to earn a policy — and the peer registry, whose foreign-boot scopes may be another kernel's live boots. An orphaned foreign-namespace directory is the user's `rm`, not the wrapper's guess.
