# ADR-0089: Carry a child-owned fact only against an obligation

## Context and Problem Statement

[ADR-0088](./ADR-0088-model-nothing-the-child-already-owns.md) removed one list of the child's settings keys. The rule it applied is broader than the record carrying it, and the repository was not clean under it: a shaped slice promised a guide only writable by importing a variable the wrapper never sets, and `prior-art.md` carried a table of child behaviour no obligation rested on. Nothing would have caught either.

## Considered Options

- Carry no child-owned fact at all.
- Carry freely, and give each fact a freshness entry in research tracking.
- Carry only against a named wrapper obligation, in a registry a gate reads.

## Decision Outcome

Chosen option: carry against a named obligation. Carrying nothing is not available — a wrapper that knows no child name cannot launch one. Carrying freely produced these defects: a tracking entry answers whether a fact drifted, never whether it belongs.

Three obligations qualify. Launch covers what the wrapper sets or passes, such as `CLAUDE_CONFIG_DIR` and `--settings`. No-collision covers the child's own spellings, which [ADR-0044](./ADR-0044-audit-wrapper-spellings-against-the-child-inventory.md) needs to prove passthrough. Scope-our-claim covers the child behaviour that stops the wrapper over-claiming its own effect.

A carry meeting none of the three is duplication and is removed. [Child facts](../reference/child-facts.yaml) is the registry; the gate fails on an unregistered carry, a dead entry, and an unregistered name in a slice entry.

The credential mechanisms outranking a selected account are scope-our-claim: the wrapper promised that account, so only it reports the promise defeated. Deleting the warning was the rejected alternative. The claim needs membership alone, so no order among them is carried.

## Consequences

- Good: shaping meets the rule before the work, rather than a reviewer after it.
- Good: a removed carry stops being a maintenance obligation at all.
- Bad: the obligations are a judgement, and a contested carry needs a question first.
- Bad: a mechanism the wrapper does not name leaves the warning quiet.

## Status

Accepted

Amends [ADR-0088](./ADR-0088-model-nothing-the-child-already-owns.md) past settings keys, and [ADR-0044](./ADR-0044-audit-wrapper-spellings-against-the-child-inventory.md) and [ADR-0057](./ADR-0057-build-the-child-environment-by-prefix-scrub-and-marker.md) by naming the obligation each already rested on.
