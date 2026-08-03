# ADR-0062: Derive the group from the controlling terminal

## Context and Problem Statement

A group owns one composed `settings.json`, its provenance sidecar, and its session metadata, so two panes running different profiles must get different groups and the same pane must get the same group on its next run. The wrapper contains no knowledge of tmux, screen, or any other multiplexer, which rules out `TMUX_PANE`, `STY`, and every emulator-supplied window variable. What remains is a ladder of kernel-visible inputs, and which inputs it uses was never specified.

## Considered Options

- The pane's own pseudo-terminal, reached through `/dev/tty`.
- Multiplexer and emulator variables, with a plain-terminal fallback.
- A login-scoped identity: `XDG_SESSION_ID`, `/proc/self/sessionid`, or logind.
- An OSC query asking the terminal to identify itself.

## Decision Outcome

Chosen option: **the controlling terminal**, first-hit-wins behind two explicit overrides, and above two headless rungs:

| # | Input                                      | Discriminates because                                                                       |
| - | ------------------------------------------ | ------------------------------------------------------------------------------------------- |
| 0 | `--session <id>`                           | The user's intent, which no input encodes                                                   |
| 1 | `CLAUDE_SESSION_GROUP`                     | A script can pin a group without editing argv                                               |
| 2 | `ttyname_r(3)` on an opened `/dev/tty`     | Every tab, split, pane, and login has exactly one controlling terminal, whatever created it |
| 3 | `getsid(0)` plus that process's start time | Works with no terminal at all, and does not fragment one shell across pipelines             |
| 4 | Random, with a warning                     | Makes derivation total                                                                      |

Rung 2 settles the multiplexer question rather than dodging it: a pane's pseudo-terminal is created once by the server at pane spawn, so it is unchanged by detach and by reattach from a different machine — which is precisely what every emulator variable fails. It is opened, never inferred from `isatty(0)`, matching [ADR-0053](./ADR-0053-read-a-confirmation-from-the-controlling-terminal.md).

Rung 3 replaces the parent process: a pipeline and a subshell share a session id but have different parents, so a parent-keyed identity would split one shell into several groups.

Login-scoped identity is rejected for granularity — it is shared by every pane of one login. An OSC query is rejected for making identity derivation blocking and input-mutating.

## Consequences

- Good: correct inside every multiplexer, and across reattach, with no multiplexer knowledge.
- Bad: a terminal slot reused by a later shell inherits the earlier group. That is the intended "same slot, same group" semantics, bounded by stale-group pruning.
- Bad: a run under `script(1)` gets its own group, because that program allocates a new terminal.

## Status

Accepted
