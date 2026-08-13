# ADR-0095: Take the first version-shaped token from the child

## Context and Problem Statement

The version floor ([ADR-0031](./ADR-0031-enforce-the-child-refresh-lock-version-floor.md)) needs a number from `claude --version`. The parser matched the child's whole line — a `claude` prefix and a space, then three dotted integers and nothing after. The child prints `2.1.220 (Claude Code)`, so the parse failed, `doctor` reported a current child as below the floor, and the same parser hard-blocked a saved-login launch. Matching a line the child owns means tracking a spelling it may change without notice.

## Considered Options

- Scan the output for the first version-shaped token and ignore everything around it.
- Track the child's current line format and update the matcher when it moves.
- Adopt a semantic-version dependency and hand it the whole line.

## Decision Outcome

Chosen option: scan for the first version-shaped token — it is the smallest fact the floor needs, and carrying less of a child-owned spelling is what [ADR-0089](./ADR-0089-carry-a-child-owned-fact-only-against-an-obligation.md) asks of every carry.

## Consequences

- Good: prefixes, suffixes, build metadata, and a `v` all parse, so a child rewording its version line does not break the floor.
- Good: the unreadable case becomes rare and can be reported as itself rather than folded into a below-floor claim.
- Bad: output whose first version-shaped token belongs to something else — a bundled runtime — would be read as the child's.
- Bad: prerelease and build suffixes are discarded rather than ordered, so `2.1.211-rc.1` reads as `2.1.211`.

## Status

Implemented

The carry is child behaviour rather than a child-owned identifier, so it is governed by [ADR-0089](./ADR-0089-carry-a-child-owned-fact-only-against-an-obligation.md) at review rather than by the identifier registry, and its perishable half stays in [research tracking](../reference/research-tracking.yaml) under [process runtime](../reference/process-runtime.md#child-version-floor). What this decision leaves carried is one sentence: the child's version output contains a dotted three-integer token. The floor value and its meaning are unchanged; only the reading of the number moves. [Slice 020](../plan/slices/020-human-first-diagnostics/README.md) enacts it.
