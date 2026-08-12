# 015 — Child fact delegation

## Goal

Make the delegation boundary checkable, so the repository carries a child-owned fact only where a wrapper obligation requires it and no later slice can be shaped against that rule.

## Appetite

2 implementation sessions.

## Core

Every child-owned fact the repository carries names the wrapper obligation that requires it, and a gate fails on one that does not.

## In scope

- One decision generalizing the settings-key rule to any child-owned name, schema, or behaviour, with the obligations that justify a carry.
- One registry of the carried facts, each naming its obligation and its sites.
- One repository gate over that registry, covering source, documentation, and slice entries.
- The delegation rule stated in the agent guidelines, where shaping reads it.
- Removal of the carries no obligation justifies, and of the tracking entries that keep them fresh.
- Reshaping the slice whose entry the rule invalidates, and routing the one contested carry.

## Out of scope

- Changing what the wrapper does at run time; every carry removed here is documentation or a tracking obligation.
- Deciding the contested ambient-credential carry, which is a decision this slice raises rather than settles.
- Re-surveying prior art, which the removals here do not need and no present boundary asks for.

## Governed by

- [AGENTS.md](../../../../AGENTS.md)
- [ADR-0088](../../../decisions/ADR-0088-model-nothing-the-child-already-owns.md)
- [ADR-0044](../../../decisions/ADR-0044-audit-wrapper-spellings-against-the-child-inventory.md)
- [Wrapper model](../../../explanation/wrapper-model.md)
- [Prior art](../../../reference/prior-art.md)
- [Research tracking](../../../reference/research-tracking.yaml)
- [Testing and quality](../../../reference/testing-and-quality.md)
- [Coding conventions](../../../reference/coding-conventions.md)
- [Development workflow](../../../guides/development-workflow.md)

## Acceptance

- Where the repository carries a child-owned name, schema, or behaviour, the registry shall name the wrapper obligation that requires it. -> repo_contracts::child_facts::every_carried_child_fact_names_its_obligation
- If a child-owned name appears at a site the registry does not list, then the gate shall fail naming that site. -> repo_contracts::child_facts::a_child_owned_name_appears_only_at_a_registered_site
- If a registry entry matches no occurrence in the tree, then the gate shall fail naming that entry. -> repo_contracts::child_facts::a_registry_entry_matches_something_in_the_tree
- When a slice entry is written, the gate shall reject an unregistered child-owned name in it. -> repo_contracts::child_facts::a_slice_entry_carries_no_unregistered_child_owned_name
- When a carry meets no obligation, the repository shall remove it rather than record it.

## Rabbit holes

- Purging every mention of the child; escape: the three obligations are the test, and launching is one of them.
- Enumerating the child's schema in the registry; escape: an entry names a fact the repository already carries, never one it might meet.
- Deciding the ambient-credential list inside the sweep; escape: route it to open questions and register the carry as contested.

## Done when

The registry gate and its negative test pass under `cargo nextest`, no unregistered child-owned name remains in the tree, the invalidated slice entry is reshaped, and `just hooks` is green.

## Revisions

None.
