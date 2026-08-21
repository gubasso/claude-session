# 033 — Dead session collection

## Goal

Give the per-terminal session directories a collector: a verb that reports each one's liveness and removes only the ones provably dead, closing the accumulation [ADR-0102](../../../decisions/ADR-0102-key-child-state-by-terminal.md) accepted as a cost.

## Appetite

2 implementation sessions.

## Core

A launch records the inputs that named its terminal, and `session clean` removes exactly the session directories whose recorded terminal is provably gone.

## In scope

- One decision recording the terminal witness at launch: the derivation preimage, written beside the session directory it names, because the sanitized terminal name is deliberately lossy and cannot be inverted.
- One decision on judgment and collection: three verdicts — live, dead, unknown — with only dead collectable and unknown never, the same reachability honesty [ADR-0108](../../../decisions/ADR-0108-share-the-child-peer-registry-across-sessions.md) applies to peers; it amends ADR-0102's accumulation consequence.
- The `session` namespace verb: `list` reports every session directory's verdict, and `clean` previews the dead set, prompts, and removes it, with `--yes` skipping the prompt and `--json` per subcommand.
- Removal under the account's write lock and the guard's symlink refusal, deleting the witness with its directory and a namespace directory only once it is empty.
- A sessions reference page owning the verb grammar, and alignment of the CLI surface, exit codes, XDG storage, session isolation, and the README where session lifetime is spelled.

## Out of scope

- Pruning the peer registry: a boot component that is not this kernel's may be another kernel's live boot, so its records are unknown rather than dead.
- Composed settings entries, whose growth curve [XDG storage](../../../reference/xdg-storage.md#composed-settings-entries) already judged too slow to earn a policy.
- Any age heuristic, which misreads both a long idle session and a fresh crash; liveness here is a fact, not an estimate.
- Automatic collection at launch or on a schedule; the verb is explicit.
- The shell predecessor's tree, whose namespace story [019](../019-original-namespace-restoration/README.md) owns.

## Governed by

- [AGENTS.md](../../../../AGENTS.md)
- [Charter](../../charter.md)
- [ADR-0006](../../../decisions/ADR-0006-place-files-by-xdg-ownership.md)
- [ADR-0051](../../../decisions/ADR-0051-let-every-surface-element-discriminate.md)
- [ADR-0060](../../../decisions/ADR-0060-lock-the-writes-that-are-not-derivable.md)
- [ADR-0061](../../../decisions/ADR-0061-protect-storage-from-accidental-local-drift.md)
- [ADR-0089](../../../decisions/ADR-0089-carry-a-child-owned-fact-only-against-an-obligation.md)
- [ADR-0102](../../../decisions/ADR-0102-key-child-state-by-terminal.md)
- [ADR-0107](../../../decisions/ADR-0107-scope-a-terminal-to-its-namespace.md)
- [ADR-0108](../../../decisions/ADR-0108-share-the-child-peer-registry-across-sessions.md)
- [ADR-0109](../../../decisions/ADR-0109-discriminate-namespaces-across-kernels.md)
- [CLI surface](../../../reference/cli-surface.md)
- [XDG storage](../../../reference/xdg-storage.md)
- [Exit codes](../../../reference/exit-codes.md)
- [Logging and output](../../../reference/logging-and-output.md)
- [Presentation](../../../reference/presentation.md)
- [Coding conventions](../../../reference/coding-conventions.md)
- [Testing and quality](../../../reference/testing-and-quality.md)
- [Development workflow](../../../guides/development-workflow.md)

## Acceptance

- When `session list` runs, every session directory shall carry a verdict and the ground that verdict is the projection of. -> sessions_gc::every_verdict_reports_the_ground_it_stands_on
- When `session clean` removes a session directory, it shall remove the namespace directory holding it only once nothing but orphan records is left inside. -> sessions_gc::an_emptied_namespace_directory_is_pruned
- When `session clean` has no terminal and no `--yes`, it shall refuse before any side effect, and a declined prompt shall exit `0`. -> sessions_gc::a_declined_prompt_removes_nothing_and_exits_zero
- When the work lands, no current document shall state that nothing prunes a session directory.

## Rabbit holes

- Inverting the sanitized terminal name instead of recording inputs; escape: the mapping is non-injective by design, so the preimage is recorded rather than recovered.
- Reading the child's peer registrations for liveness; escape: no ADR-0089 obligation covers consuming that schema, so the wrapper judges from its own record.
- Growing `clean` filters — per account, per age, per rung — before a need exists; escape: the dead set is the whole surface.

## Done when

A launch records its witness, `session list` tells the three states apart, `session clean` removes exactly the dead after a prompt, every owner page tells the new lifetime story, and `just hooks` is green.

## Revisions

2026-08-21: the acceptance line about an unchanged record left unwritten was removed, superseded by [036](../036-agent-keyed-sessions/README.md): a session directory belongs to one agent run, so there is no relaunch into it and nothing to leave unwritten.

2026-08-21: the two acceptance lines asserting three verdicts, and collection of the dead alone, were superseded by [035](../035-total-session-accounting/README.md) and replaced by two this slice's own work still holds. What was learned is that a third verdict nothing could collect was not a boundary but an accumulation: the directory it was invented for could never be claimed by any later launch either, so keeping it left the tree with entries no verb could ever reach.
