# ADR-0017: Declare a human-facing CLI

## Context and Problem Statement

Several rules already written down rest on a premise never stated: that `claude-session` is built for a person at a terminal rather than for a program parsing its output. The terminal-output module's name, structured output being opt-in, and the diagnostic stream contract all assume it. Left unstated, the next contributor has no way to settle the questions it answers.

## Considered Options

- Human-facing — a person at a terminal is the primary consumer.
- Machine-facing — a program is, and prose output is the accommodation.
- Decide per command, or infer at runtime from whether standard output is a terminal.

## Decision Outcome

Chosen option: human-facing. This is a wrapper a developer runs interactively, many times a day, whose main mode is handing a terminal session to a child that is itself interactive.

Three consequences follow, and settling them is the point of declaring it. Machine-readable output is opt-in, through an explicit flag, never the default. The terminal-output module is named and shaped for human rendering — `ui/`, not a protocol module. Diagnostics are mirrored to standard error by default, because a person needs to see them without enabling anything first.

This is a design-time category, not a runtime `isatty()` flip. Detecting a terminal changes colour and progress rendering and nothing else; it never changes which format a command emits. Output that reshapes itself when piped is precisely the behaviour that makes a CLI unscriptable.

## Consequences

- Good: output defaults, module naming, and stream behaviour follow from one stated premise instead of three unstated ones.
- Good: piping a command's output does not change its format.
- Bad: script authors must pass the flag explicitly rather than getting structure by default.
- Bad: a future machine-first consumer would need this record amended, not merely a flag added.

## Status

Accepted

Amended by [ADR-0024](./ADR-0024-machine-output-is-a-per-verb-flag.md), which settles the flag's spelling and placement as a verb-level `--json`. The opt-in decision recorded here is unchanged.
