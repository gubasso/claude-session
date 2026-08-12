# 016 — Ambient credential scope

## Goal

Settle what the wrapper may observe about credentials it does not manage, so the one carry the delegation sweep left contested becomes either justified or gone.

## Appetite

1 implementation session.

## Core

The wrapper warns that its own account selection is defeated, and carries only what that claim needs.

## In scope

- One decision closing the contested carry, recording the option of deleting the warning rather than shipping it.
- Reduction of the carried list to what the wrapper's own claim rests on, dropping any assertion of the child's relative precedence from source and comments.
- Re-registration of the affected entries in [child facts](../../../reference/child-facts.yaml) against the obligation the decision names.
- Alignment of the account reference where it describes the child's arbitration rather than the wrapper's claim.

## Out of scope

- Detecting mechanisms that live in the child's settings rather than the environment, which the wrapper would have to author a settings document to clear.
- Enumerating mechanisms the wrapper has no present reason to observe.
- Changing how the warning is presented, which [presentation](../../../reference/presentation.md) owns and this decision does not touch.

## Governed by

- [AGENTS.md](../../../../AGENTS.md)
- [ADR-0089](../../../decisions/ADR-0089-carry-a-child-owned-fact-only-against-an-obligation.md)
- [ADR-0088](../../../decisions/ADR-0088-model-nothing-the-child-already-owns.md)
- [Accounts](../../../reference/accounts.md)
- [Child facts](../../../reference/child-facts.yaml)
- [Presentation](../../../reference/presentation.md)
- [Coding conventions](../../../reference/coding-conventions.md)
- [Testing and quality](../../../reference/testing-and-quality.md)
- [Development workflow](../../../guides/development-workflow.md)

## Acceptance

- When a credential mechanism outranks the account the wrapper selected, the wrapper shall warn that its own selection is defeated.
- Where the repository carries a child-owned credential name, the registry shall name an obligation rather than a contested state.
- Where the wrapper does not arbitrate between mechanisms, the repository shall assert no order among them.
- When the decision lands, open questions shall carry no entry for the ambient carry.

## Rabbit holes

- Reproducing the child's arbitration so the warning can rank what it found; escape: the claim is that something outranks the selection, which membership answers.
- Chasing completeness of the mechanism list; escape: a mechanism the wrapper does not observe makes the warning quiet, not wrong, and the decision records that cost.
- Reopening the environment construction the exec owns; escape: the warning reads the environment and changes nothing in it.

## Done when

The decision is recorded and applied, no entry in the registry is contested, the open question is closed with its exit, and `just hooks` is green.

## Revisions

None.
