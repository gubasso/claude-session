# ADR-0115: Return to the project namespace

## Context and Problem Statement

The shell predecessor that owned `claude-session` under the config and state bases and the `CLAUDE_SESSION_` environment prefix is retired and uninstalled. [ADR-0092](./ADR-0092-namespace-apart-from-the-predecessor.md) moved every name the wrapper reads or writes to `claude-session-rs` to avoid that collision, and scheduled its own reversal for the day the collision ended. That day has arrived, so the wrapper now answers to a spelling that names nothing it needs to avoid, while the project it ships is called `claude-session`.

## Considered Options

- Return every name to `claude-session` together.
- Keep `claude-session-rs` permanently, since it works and moving costs a migration.
- Return the paths but keep the command name, so an installed shell completion does not change.

## Decision Outcome

Chosen option: return every name together — the four XDG namespace directories, the log filename, the project configuration filename, the command name, the cargo binary name, and the environment prefix with its scrub. A partial return is the failure ADR-0092 already named: paths alone leave the scrub deleting variables the wrapper never sets, and a command name alone leaves the page describing a binary nobody can type.

Keeping the coexistence spelling was rejected because it makes a temporary avoidance permanent, and every reader who asks why the tool is named twice gets a program that no longer exists as the answer.

Nothing reads the coexistence namespace at run time. State written under it is moved once by an operator, not read forever by the wrapper — a compatibility read would make the coupling permanent in the name of ending it.

## Consequences

- Good: the wrapper's on-disk identity, its command name, and its project name are one spelling, and the environment prefix it scrubs is the one it sets.
- Bad: state written under the coexistence namespace is stranded until an operator moves it, and an installed completion registered against the old name keeps completing nothing.

## Status

Implemented

Enacted by [`src/domain/paths.rs`](../../src/domain/paths.rs), [`src/services/child.rs`](../../src/services/child.rs), [`src/cli.rs`](../../src/cli.rs), [`src/config/project.rs`](../../src/config/project.rs), and [`src/logging.rs`](../../src/logging.rs). Supersedes [ADR-0092](./ADR-0092-namespace-apart-from-the-predecessor.md), returning every literal it moved.
