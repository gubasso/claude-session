# ADR-0086: Emit one man page to standard output

## Context and Problem Statement

[ADR-0016](./ADR-0016-ship-man-pages.md) specified `man` as writing roff to standard output or to a named directory, the second form producing one page per verb so a packager could render at build time. The CLI-artifacts rung builds the verb and finds no packager: nothing in this repository or its release path consumes a page set, and the directory form would be the only surface where the wrapper writes a file the user named rather than one it owns.

## Considered Options

- Ship both forms as specified.
- Ship standard output alone, and restore a destination when a packager needs one.
- Ship the directory form alone, since a packager is the eventual reader.

## Decision Outcome

Chosen option: standard output alone — a second form that no present use discriminates is speculative surface, which [ADR-0051](./ADR-0051-let-every-surface-element-discriminate.md) rejects.

`claude-session man > claude-session.1` still installs, so the packaging path ADR-0016 wanted is reachable; what is deferred is the per-verb page set. Dropping the destination also keeps every file the wrapper writes wrapper-owned, which is the premise [exit codes](../reference/exit-codes.md) rests on when it declines to mint `CantCreat`. Restoring `--out-dir` later is an append, and the option is recorded here so it is restored knowingly.

## Consequences

- Good: no user-named write path, so no missing-directory branch and no new failure class.
- Good: the verb is one buffer to one stream, which is the whole of its contract.
- Bad: per-verb pages are unreachable, so `man claude-session-account` does not exist and that detail lives only in `account --help`.
- Bad: a packager wanting the full set must wait for a slice that restores a destination.

## Status

Accepted

Implemented by the CLI-artifacts rung in `src/commands/man.rs`. Narrows [ADR-0016](./ADR-0016-ship-man-pages.md), which keeps its status and records the amendment.
