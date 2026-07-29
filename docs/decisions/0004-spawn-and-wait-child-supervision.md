# ADR-0004: Spawn and wait rather than exec

## Context and Problem Statement

The wrapper must run the child and be transparent about it. Replacing its own process image with the child's is the cheapest way to be transparent: no signal forwarding, no exit-status translation, no extra process. But the wrapper also owns work that can only run _after_ the child exits — syncing credential and project-trust state out of the isolated session directory.

## Considered Options

- `exec` the child, replacing the wrapper's process image.
- Spawn the child, wait for it, then run post-flight work.
- `exec`, and do everything the wrapper needs before the replacement.

## Decision Outcome

Chosen option: **spawn and wait** — `exec` never returns, so post-flight sync-back would never run, and that work cannot move earlier because it depends on what the child did.

Having chosen to stay alive, the wrapper must be **behaviourally indistinguishable** from `exec` in everything observable: the same exit status, terminal behaviour, and response to an interrupt.

That is met by a specific topology. The child **shares the wrapper's foreground process group**, so a terminal-generated signal reaches both and the wrapper must **not** forward it — forwarding double-delivers, and a child counting interrupts misreads one keypress as two. Forwarding is deliberately partial: only what the terminal does not broadcast. See [process runtime](../reference/process-runtime.md).

## Consequences

- Good: post-flight sync-back runs, which is the whole reason for the choice.
- Good: sharing the process group keeps job control, resize, and interrupt working without the wrapper mediating them.
- Bad: the wrapper owns signal handling, status mapping, and reaping by hand, each a way to be subtly wrong.
- Bad: "forward every signal" — the obvious implementation — is a bug here, so the matrix must be followed rather than reasoned out afresh.
- Bad: a post-flight failure must not change the child's exit status, making error handling after the wait asymmetric with before it. See [exit codes](../reference/exit-codes.md).

## Status

Accepted
