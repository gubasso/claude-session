# ADR-0034: Exit `1` when `doctor --strict` promotes a warning

## Context and Problem Statement

The check catalog's soft checks are legitimately ignorable — a degraded feature the user does not use, which is why a soft failure never gates. A CI job that wants them fatal has no way to say so, and reading `summary.warned` out of `doctor --json` puts a JSON parser inside a step whose whole job is to be a gate.

## Considered Options

- No flag; a caller parses `doctor --json` and decides for itself.
- `--strict` exiting an existing `sysexits` code.
- `--strict` exiting a bare `1`.

## Decision Outcome

Chosen option: **a bare `1`**, and it is the only one the wrapper emits.

Every other code names a category: something was unavailable, misconfigured, unreadable. A promoted warning is none of those — the run succeeded and every check reported. What failed is a policy the caller supplied at the call site, so borrowing a `sysexits` category would claim a diagnosis the wrapper never made. `1` is the conventional "the thing you asked about is not true", which is exactly the claim being made.

The flag changes no check, no severity, and no output — only the exit. Absent it the exit is unchanged, so `--strict` can never make a passing catalog fail. It is available on `doctor` alone, because `doctor` is the only verb with a warning to promote. Appending the code is what [ADR-0033](./ADR-0033-append-fresh-exit-codes.md) permits; sanctioning a _bare_ code in a taxonomy of categories is the decision recorded here.

## Consequences

- Good: a CI gate is one flag, and `doctor` stays the single place health is defined.
- Good: `1` gains a documented meaning instead of remaining an unexplained hole.
- Bad: "every code names a category" now has one deliberate exception to explain.
- Bad: a caller that treats any non-zero as broken sees no difference, so the flag helps only a caller that reads the code.

## Status

Accepted
