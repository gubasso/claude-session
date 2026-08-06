# ADR-0030: Use account login for wrapper authentication

## Context and Problem Statement

The wrapper needs one idempotent entry point for stored subscription authentication. The child already owns `claude auth ...`, so claiming `auth` would create an immediate passthrough collision.

## Considered Options

- Use `account login [name]` with token-source flags.
- Keep separate planned `account add` and `account refresh` commands.
- Claim a wrapper-owned `auth` command.

## Decision Outcome

Chosen option: `account login [name]` — it creates an account on the first successful login and replaces its selected authentication mode on later successful runs.

`--token` selects long-lived subscription-token ingestion; `--stdin` selects its non-interactive source. Native authentication remains reachable verbatim, including `claude-session --account work -- auth login`. An in-TUI `/login` remains child-owned and cannot be intercepted.

## Consequences

- Good: creation and reauthentication share one idempotent contract.
- Good: the wrapper does not mirror or block the child's current `auth` subtree.
- Bad: callers of the superseded planned surface must use `account login`.
- Bad: slash-command behavior can only be reported or warned about outside the child TUI.

Claiming `auth` would otherwise force native callers through `--`, contrary to [ADR-0002](./ADR-0002-verbatim-argv-passthrough.md) and the small surface in [ADR-0003](./ADR-0003-reserve-a-small-wrapper-cli-surface.md). See [ADR-0021](./ADR-0021-fail-closed-without-a-terminal.md).

## Status

Accepted

Amended by [ADR-0079](./ADR-0079-compose-every-overlapping-surface-with-the-child.md), which restates this rename as an instance of a rule: `auth` changes stored state, so composing was never available to it.
