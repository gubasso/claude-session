# ADR-0005: Split the exit-code taxonomy at the spawn

## Context and Problem Statement

A wrapper has two kinds of failure: its own, before the child runs, and the child's, afterwards. Scripts wrap this wrapper, so its exit status is a public interface. A single consistent scheme would be tidy, but the two kinds of failure have genuinely different audiences.

## Considered Options

- Use BSD `sysexits` codes throughout, translating the child's status into that scheme.
- Pass the child's status through for everything, and use generic failure codes for the wrapper's own errors.
- Split at the spawn: `sysexits` before the child runs, verbatim child status afterwards.

## Decision Outcome

Chosen option: split at the spawn — a wrapper that translates its child's exit code breaks every script wrapping it, and a wrapper with only generic codes for its own failures is undiagnosable.

Before the child runs, each failure class maps to a `sysexits` value, plus the shell conventions `126` for found-but-not-executable and `127` for not-found. Once the child runs, its status is reproduced: an exit code passes through unchanged, and signal death is reproduced by re-raising the signal on the wrapper itself, falling back to `128 + N`.

Two properties make it usable. The mapping is exhaustive over a closed error enum with no catch-all arm, so adding a variant without a code fails the build. And every wrapper-originated failure carries a stable `err.kind` on standard error, which is what a script matches on. See [exit codes](../reference/exit-codes.md).

## Consequences

- Good: the no-catch-all rule makes an unmapped variant a build failure rather than a silent generic error.
- Bad: the two regimes share a numeric range — `64` is both a `sysexits` value and a legal child exit code — so a caller distinguishing them must read `err.kind` on standard error.
- Bad: the matrix is append-only. Renaming an `err.kind` or remapping a code is breaking, and needs a superseding decision.

## Status

Accepted

Amended by [ADR-0033](./ADR-0033-append-fresh-exit-codes.md) for unused numbers, [ADR-0034](./ADR-0034-exit-one-when-doctor-strict-promotes-a-warning.md) for `doctor --strict`, and [ADR-0068](./ADR-0068-spawn-the-child-as-a-subroutine.md) for subroutine spawns. Realized by [ADR-0035](./ADR-0035-convert-the-typed-error-to-a-code-once.md).
