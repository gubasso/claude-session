# 035 — Total session accounting

## Goal

Close the gap [ADR-0111](../../../decisions/ADR-0111-collect-only-the-provably-dead-session.md) left open: a session directory the wrapper could not decide was kept forever, with no verb that could ever remove it, so the tree accumulated directories nothing accounted for and nothing could clear.

## Appetite

1 implementation session.

## Core

A session directory is kept only while this run can prove its terminal is still there, and `session clean` removes every one that is not.

## In scope

- One decision superseding ADR-0111: the session tree is the wrapper's own, so a directory in it is a session it can account for or it is garbage, with no third state.
- `Verdict::collectable` as the single owner of the policy, so the survey, the collector, the prompt, and both renderers cannot disagree about what the verb takes.
- An `orphaned` verdict for the record naming the alias every process shares, which no name this version issues can ever land on again, so the report can say what a reader loses rather than pretending the directory is undecidable.
- One judgment shared by the survey and the re-judgment inside the removal lock, so a directory carrying no readable record is judged rather than skipped by the collector that was meant to take it.
- A refusal when the run cannot name its own namespace, because a run that has placed no record would read every directory as unaccounted for and empty the tree.
- A confirmation grouped by verdict, naming every path before it asks once.
- Report repair the same reading found: a row that outlives its child said so nowhere, and a next command wrapped mid-way cannot be copied back into a shell.

## Out of scope

- What a session is keyed by. The tty rung is what makes `orphaned` and `unknown` reachable at all, and replacing it is [036](../036-agent-keyed-sessions/README.md)'s to settle.
- The peer registry, whose foreign-boot scopes may be another kernel's live boots.
- Any age heuristic; liveness here is a fact, not an estimate.
- Automatic collection at launch or on a schedule; the verb stays explicit.

## Governed by

- [AGENTS.md](../../../../AGENTS.md)
- [Charter](../../charter.md)
- [ADR-0051](../../../decisions/ADR-0051-let-every-surface-element-discriminate.md)
- [ADR-0061](../../../decisions/ADR-0061-protect-storage-from-accidental-local-drift.md)
- [ADR-0102](../../../decisions/ADR-0102-key-child-state-by-terminal.md)
- [ADR-0110](../../../decisions/ADR-0110-record-the-terminal-witness-at-launch.md)
- [ADR-0111](../../../decisions/ADR-0111-collect-only-the-provably-dead-session.md)
- [Sessions](../../../reference/sessions.md)
- [Presentation](../../../reference/presentation.md)
- [Coding conventions](../../../reference/coding-conventions.md)
- [Testing and quality](../../../reference/testing-and-quality.md)
- [Development workflow](../../../guides/development-workflow.md)

## Acceptance

- When `session clean` runs, it shall remove every session directory this run cannot prove is live, and shall leave every one it can. -> sessions_gc::session_clean_removes_everything_not_proven_live
- If a witness is reachable only through a symbolic link, then the wrapper shall neither read it nor let it keep the directory it sits beside. -> sessions_gc::a_symlinked_witness_speaks_for_nothing_and_saves_nothing
- When the run cannot name the namespace its own session directories are scoped by, `session clean` shall refuse before any side effect.
- When the work lands, no current document shall state that an undecidable session directory is kept.

## Rabbit holes

- Rekeying a session off the terminal while fixing what to do with the directories; escape: the policy is decidable under either key, so it lands first and [036](../036-agent-keyed-sessions/README.md) carries the key.
- Collapsing the four verdicts into `live` and `collectable` once one predicate owns the policy; escape: the words differ in what a reader loses by collecting, which is the sentence the prompt owes them.
- Growing `clean` filters — per account, per verdict, per age — to soften the policy; escape: the collectable set is the whole surface, and a run that can prove nothing refuses rather than filtering.

## Done when

`session list` tells the four states apart, `session clean` removes everything it cannot prove is live after one grouped prompt, a run that cannot place itself refuses, every owner page tells the new policy, and `just hooks` is green.

## Revisions

2026-08-21: the two acceptance lines naming the `orphaned` verdict were removed, superseded by [036](../036-agent-keyed-sessions/README.md). The verdict named the residue of a terminal-keyed launch; keying a session to its agent retires the ground it stood on, so the condition no longer exists to be reported. The collection policy this slice decided is unchanged and still governs.
