# ADR-0110: Record the terminal witness at launch

## Context and Problem Statement

A session directory is named by a deliberately lossy mapping of its terminal: bytes outside the identifier grammar collapse, and an irreversible device carries a fingerprint. Once the launch exits, nothing can recover which device or session leader the name stood for, so no later run can ask whether that terminal still exists — which is exactly the question collection has to answer.

## Considered Options

- Invert the sanitized name back to a device path.
- Read the child's peer registrations, which are pid-keyed and already present.
- Keep a wrapper process alive to hold a liveness lock.
- Record the derivation preimage at launch and re-ask the question later.

## Decision Outcome

Chosen option: record the preimage — the only option that keeps naming and judging inside one owner. Each launch writes a versioned witness beside the session directory it names, `accounts/<account>/sessions/<namespace>/.<terminal>.witness.json`, mode `0600`, following [ADR-0087](./ADR-0087-keep-the-credential-lock-beside-the-account.md)'s precedent of placing a wrapper artifact beside the directory whose fate it must survive. It holds the rung, the rung's own witness — the device path bytes, or the session leader's id and start time — the namespace component, and the boot identifier. The write is atomic and skipped when the recorded bytes already match, so repeated launches of one terminal converge instead of racing.

Inversion was rejected because the mapping is non-injective by design and a fingerprint does not invert. The registry was rejected because no [ADR-0089](./ADR-0089-carry-a-child-owned-fact-only-against-an-obligation.md) obligation covers consuming a child-owned schema. A held lock was rejected because [ADR-0084](./ADR-0084-exec-the-child-instead-of-supervising-it.md) leaves no wrapper process to hold it.

## Consequences

- Good: liveness becomes derivable later from wrapper-owned facts alone.
- Good: a directory predating its witness is judged unknown rather than misjudged.
- Bad: one more write on the launch path.
- Bad: the witness name must move with every rename slice 019 performs.

## Status

Implemented

Enacted in [the session service](../../src/services/session.rs). Judged by [ADR-0111](./ADR-0111-collect-only-the-provably-dead-session.md). Shaped by [033](../plan/slices/033-dead-session-collection/README.md).

Amended by [ADR-0113](./ADR-0113-key-a-session-to-its-running-agent.md): the witness records the agent process, and version `1` is no longer read.
