# ADR-0111: Collect only the provably dead session

## Context and Problem Statement

Session directories accumulate: a terminal slot that never returns, a container namespace never recreated, a headless leader long exited. [ADR-0102](./ADR-0102-key-child-state-by-terminal.md) accepted the growth and pruned nothing. With [ADR-0110](./ADR-0110-record-the-terminal-witness-at-launch.md)'s witness recorded, the wrapper can finally ask whether a directory's terminal still exists — but on a shared state tree another kernel's sessions are visible without being decidable.

## Considered Options

- Two verdicts: a session is live or collectable.
- An age policy over directory timestamps.
- Three verdicts, with only the dead collectable.

## Decision Outcome

Chosen option: three verdicts. A witness whose namespace component matches this run's is decidable: the tty rung is live while its device path exists, and the session-leader rung while its process id and start time match under the recorded boot — a foreign boot is dead, since no process survives its kernel. Everything else — a foreign namespace, a missing or unreadable witness — is unknown, and unknown is never collected: the honesty [ADR-0108](./ADR-0108-share-the-child-peer-registry-across-sessions.md) applies to peers, because absence of evidence on a shared tree is not death. The tty rung ignores boot deliberately: a reopened slot after reboot is the same slot, and slot reuse is intended.

`session list` reports the three states. `session clean` previews the dead set, prompts, removes it under the account's write lock, and removes a namespace directory only once empty; `--yes` skips the prompt, declining exits `0`, and nothing collects automatically.

Two verdicts were rejected because they let a host confidently delete a running container's sessions. An age policy was rejected because it misreads a long idle session and a fresh crash in opposite directions when liveness is checkable.

## Consequences

- Good: collection is safe exactly where the tree is shared, where it is dangerous.
- Good: a pre-witness directory self-heals into decidability at its next launch.
- Bad: an orphaned foreign-namespace directory is never the wrapper's to collect; removing it stays the user's `rm`.

## Status

Superseded

Superseded by [ADR-0112](./ADR-0112-keep-only-the-session-proven-live.md). Amends [ADR-0102](./ADR-0102-key-child-state-by-terminal.md). Shaped by [033](../plan/slices/033-dead-session-collection/README.md).
