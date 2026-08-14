# ADR-0102: Derive the child state directory from the controlling terminal

## Context and Problem Statement

One account points every terminal at one `CLAUDE_CONFIG_DIR`. Three files inside it are keyed by nothing. Concurrent panes interleave writes to `.claude.json`, which the child recovers from a backup, losing whatever the interleaving dropped; `history.jsonl` mixes every pane's prompts into one bounded recall window; `.credentials.json` is refreshed by whichever pane reaches expiry first. Everything else the child writes there — transcripts, session registrations, per-session environments — is already keyed by an identifier and does not collide. [ADR-0065](./ADR-0065-retire-the-terminal-group.md) retired the terminal axis when nothing read it.

## Considered Options

- Leave one directory per account and accept the collisions.
- Key the whole account tree by terminal, as the shell predecessor does.
- Key only the child state directory by terminal, sharing the credential and the projects tree.

## Decision Outcome

Chosen option: key only the child state directory. `CLAUDE_CONFIG_DIR` becomes `accounts/<account>/sessions/<terminal>/`. The saved login stays at `accounts/<account>/config/`, reached through the child's own credential-store variable ([ADR-0104](./ADR-0104-share-one-credential-store.md)), and `projects/` is shared back through a declared link ([ADR-0103](./ADR-0103-permit-a-declared-link.md)).

Splitting the whole tree was measured on the predecessor rather than argued: `projects/<project>/memory/` splits with it, and one machine accumulated fifty-two memory directories, two of them for a single repository, neither able to read the other.

The derivation is the ladder [ADR-0062](./ADR-0062-derive-the-group-from-the-controlling-terminal.md) recorded, minus the two rungs ADR-0065 retired: the controlling terminal, then the session leader with its start time, then a refusal. Nothing but the path component reads the result.

[ADR-0064](./ADR-0064-key-composed-settings-by-profile-and-input-digest.md) is untouched. Composed settings stay keyed by profile and input digest, and the terminal keys nothing there.

## Consequences

- Good: the three unkeyed files stop colliding, and per-project memory stays whole.
- Good: one saved login per account survives, so the child's refresh lock still guards one file.
- Bad: session directories accumulate and nothing prunes them.
- Bad: a reused terminal slot inherits the earlier directory, which is the intended semantics and was already recorded against ADR-0062.

## Status

Implemented

Supersedes [ADR-0065](./ADR-0065-retire-the-terminal-group.md). Enacted in [the ladder](../../src/domain/terminal.rs) and [the session service](../../src/services/session.rs). Shaped by [028](../plan/slices/028-per-terminal-session-isolation/README.md).
