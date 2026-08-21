# ADR-0112: Keep only the session proven live

## Context and Problem Statement

[ADR-0111](./ADR-0111-collect-only-the-provably-dead-session.md) made `unknown` uncollectable, so a directory the wrapper could not decide stayed forever. The residue of the pre-witness naming defect showed the cost: nothing could claim it, nothing could remove it, and every `session list` reported it as an open question the user had no verb for.

## Considered Options

- Keep unknown, and document the user's `rm`.
- A fourth verdict for the alias residue alone, still keeping unknown.
- Keep only what this run proves live; everything else is collectable.

## Decision Outcome

Chosen option: keep only the proven live. The session tree is the wrapper's own, so a directory in it is either a session the wrapper can account for or garbage, with no third state worth carrying. `Verdict::collectable` owns the policy, and `live` — a recorded device that still exists, a recorded leader still running under the recorded boot — is the only verdict that keeps a directory.

Four words survive, because they differ in what collecting costs a reader: `dead` closed, `orphaned` was never owned and can never be claimed, `unknown` could not be accounted for at all. Each row says which, and the prompt groups by it before asking once.

One refusal guards the policy. A run that cannot name its namespace has placed no record, so every directory reads as unaccounted for; `session clean` refuses rather than emptying a tree it cannot see.

Documenting the user's `rm` was rejected as the wrapper declining to clean up after its own defect. A verdict for the alias alone was rejected because it left `unknown` accumulating for the same reason.

## Consequences

- Good: every directory has a verb, so nothing accumulates that nothing can remove.
- Good: one predicate owns collectability, rather than a filter per caller.
- Bad: on a state tree shared with another kernel, a foreign namespace's sessions are collectable here, and that side loses them.

## Status

Implemented

Supersedes [ADR-0111](./ADR-0111-collect-only-the-provably-dead-session.md). Enacted in [the judgment](../../src/domain/witness.rs), [the collector](../../src/services/session/gc.rs), and [the verb](../../src/commands/session.rs). Shaped by [035](../plan/slices/035-total-session-accounting/README.md).
