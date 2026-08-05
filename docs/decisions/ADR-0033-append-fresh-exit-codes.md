# ADR-0033: Append fresh exit codes rather than reusing the set

## Context and Problem Statement

[ADR-0005](./ADR-0005-exit-code-taxonomy.md) fixed the taxonomy and [exit codes](../reference/exit-codes.md) recorded it as append-only — but spelled append-only as "a new failure class gets a code drawn from the existing table". That freezes the numeric set at whatever the first draft happened to need. The first condition it blocked was real: a `doctor` mode promoting warnings to a failure had no code to exit with, and the rule rejected the feature rather than the phrasing.

## Considered Options

- Keep the closed set; a new failure class reuses an existing code.
- Append fresh, previously-unused numbers, with every assigned code permanent.
- Abandon the fixed set and let each verb mint its own codes.

## Decision Outcome

Chosen option: append fresh numbers — a rule that cannot grow is a freeze rather than a stability guarantee, and it lets the taxonomy's phrasing decide which features exist.

What needs to be stable is that a code's meaning never changes. Adding an unused number breaks nothing: a script matching `78` still matches `78`, and one testing non-zero is unaffected. Reusing a code for a second, unrelated class is what breaks callers, because it silently widens what an existing branch catches.

Three rules replace the one. A code's meaning is permanent and never reassigned. A new failure class takes an unused number, preferring the `sysexits` category that already names it. Consumers branch on `0` versus non-zero, or on a documented code — never on the assumption that no new code can appear.

## Consequences

- Good: `EX_TEMPFAIL` (75) becomes available for a contended runtime lock, until now miscategorised as `Unavailable`.
- Bad: the set grows, so a consumer that enumerated it must tolerate numbers it has not seen.
- Bad: "prefer the category that already names it" is judgement, and two reviewers can disagree about whether a class is genuinely new.

## Status

Accepted

Amends [ADR-0005](./ADR-0005-exit-code-taxonomy.md) — the split at the spawn and the exhaustive no-catch-all mapping are unchanged; only the stability rule changes.
