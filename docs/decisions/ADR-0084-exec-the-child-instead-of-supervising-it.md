# ADR-0084: Exec the child instead of supervising it

## Context and Problem Statement

Spawning the child and waiting for it obliges the wrapper to own signal forwarding, exit-status translation, and job-control mirroring by hand, and the naive implementation of each is a bug rather than a rough edge. [ADR-0004](./ADR-0004-spawn-and-wait-child-supervision.md) accepted that cost so credential sync-back could run after the child; [ADR-0025](./ADR-0025-share-one-native-login-per-account.md) removed that rationale. The only after-child obligation left is the account last-used marker, and a marker recording which account was selected is equally correct written at launch.

## Considered Options

- Replace the wrapper's process image with the child's, after every wrapper obligation has run.
- Keep spawn-and-wait with the full partial-forwarding matrix.
- Keep spawn-and-wait and forward nothing, as a shell script does.

## Decision Outcome

Chosen option: exec — the wrapper's job ends once the child is running correctly, and a replaced image makes argv, streams, exit status, signal death, and job control the child's own facts rather than contracts the wrapper reimplements.

Every obligation therefore runs before the replacement: child resolution and both recursion guards, the account directories, the composed settings entry, the argument prefix, the child environment, and the log flush. Forwarding nothing while staying alive was rejected as the worst of both: the wrapper would die of Ctrl-C before the child and orphan it on a targeted `SIGTERM`.

## Consequences

- Good: no signal handling, no status mapping, no supervision to be subtly wrong about, and one fewer dependency.
- Good: `ps` and `kill` name the program the user is running, because the process id does not change.
- Bad: nothing can run after the child, so a future obligation of that shape needs a record reversing this one.
- Bad: a failed exec is reported on standard error alone, since the log sink is flushed before it.

## Status

Implemented

Enacted by [`src/adapters/process.rs`](../../src/adapters/process.rs) and [`src/main.rs`](../../src/main.rs). Supersedes [ADR-0004](./ADR-0004-spawn-and-wait-child-supervision.md). Amends [ADR-0005](./ADR-0005-exit-code-taxonomy.md), [ADR-0058](./ADR-0058-behave-as-stock-claude-by-default.md), and [ADR-0080](./ADR-0080-order-the-boundary-as-report-flush-exit.md), each of which assumed a wrapper alive beside its child. The exact sequence is in [process runtime](../reference/process-runtime.md).
