# ADR-0089: Carry a child-owned fact only against an obligation

## Context and Problem Statement

[ADR-0088](./ADR-0088-model-nothing-the-child-already-owns.md) removed one list of the child's settings keys. The rule it applied is broader than the record that carries it, and the repository was not clean under it: a shaped slice promised a proxy guide that could only be written by importing a variable the wrapper never sets, and `prior-art.md` carried a table of child behaviour no wrapper obligation rested on. Nothing would have caught either, because there was no statement of the rule outside a settings-scoped record and nothing enforcing it.

## Considered Options

- Carry no child-owned fact at all.
- Carry freely, and give each fact a freshness entry in research tracking.
- Carry only against a named wrapper obligation, in a registry a gate reads.

## Decision Outcome

Chosen option: carry against a named obligation. Carrying nothing is not available — a wrapper that knows no child name cannot launch one. Carrying freely is what produced these defects: a tracking entry answers whether a fact drifted, never whether it should be here.

Three obligations qualify. Launch covers what the wrapper sets or passes, such as `CLAUDE_CONFIG_DIR` and `--settings`. No-collision covers the child's own spellings, which [ADR-0044](./ADR-0044-audit-wrapper-spellings-against-the-child-inventory.md) needs to prove passthrough. Scope-our-claim covers the child behaviour that stops the wrapper over-claiming its own effect.

A carry meeting none of the three is duplication of the child's documentation and is removed. [Child facts](../reference/child-facts.yaml) is the registry; the gate fails on an unregistered carry, a dead entry, and an unregistered name in a slice entry, which is where the invalid slice came from.

## Consequences

- Good: shaping meets the rule before the work, rather than a reviewer after it.
- Good: a removed carry stops being a maintenance obligation, not just an unlisted one.
- Bad: the obligations are a judgement, and a genuinely contested carry needs a question rather than a verdict.

## Status

Accepted

Amends [ADR-0088](./ADR-0088-model-nothing-the-child-already-owns.md) by generalizing it past settings keys, [ADR-0044](./ADR-0044-audit-wrapper-spellings-against-the-child-inventory.md) and [ADR-0057](./ADR-0057-build-the-child-environment-by-prefix-scrub-and-marker.md) by naming the obligation each already rested on.
