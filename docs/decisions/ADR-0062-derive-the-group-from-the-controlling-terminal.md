# ADR-0062: Derive the group from the controlling terminal

## Context and Problem Statement

A group owns one composed `settings.json`, its provenance sidecar, and its session metadata, so two panes running different profiles must get different groups and the same pane must get the same group on its next run. The wrapper contains no knowledge of tmux, screen, or any other multiplexer, which rules out `TMUX_PANE`, `STY`, and every emulator-supplied window variable. What remains is a ladder of kernel-visible inputs, and which inputs it uses was never specified.

## Considered Options

- The pane's own pseudo-terminal, reached through `/dev/tty`.
- Multiplexer and emulator variables, with a plain-terminal fallback.
- A login-scoped identity: `XDG_SESSION_ID`, `/proc/self/sessionid`, or logind.
- An OSC query asking the terminal to identify itself.

## Decision Outcome

Chosen option: first-hit-wins from `--session <id>`, `CLAUDE_SESSION_GROUP`, `ttyname_r(3)` on opened `/dev/tty`, `getsid(0)` plus the leader's start time, then random with a warning. The first two express intent; the terminal distinguishes panes; the session rung handles headless pipelines; random makes derivation total.

The terminal settles multiplexers without naming them: a pane's pseudo-terminal survives detach and reattach. It is opened, never inferred from `isatty(0)`, matching [ADR-0053](./ADR-0053-read-a-confirmation-from-the-controlling-terminal.md).

The session rung avoids splitting pipelines that have different parents.

Login-scoped identity is rejected for granularity — it is shared by every pane of one login. An OSC query is rejected for making identity derivation blocking and input-mutating.

## Consequences

- Bad: a terminal slot reused by a later shell inherits the earlier group. That is the intended "same slot, same group" semantics, bounded by stale-group pruning.
- Bad: a run under `script(1)` gets its own group, because that program allocates a new terminal.

## Status

Superseded

Superseded by [ADR-0065](./ADR-0065-retire-the-terminal-group.md) — settings are keyed by profile and input digest under [ADR-0064](./ADR-0064-key-composed-settings-by-profile-and-input-digest.md), so the group has no consumer left to derive.
