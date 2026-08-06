# ADR-0035: Convert the typed error to a process code once, at `main`

## Context and Problem Statement

[ADR-0008](./ADR-0008-layered-error-architecture.md) fixed the error-type stack and [ADR-0005](./ADR-0005-exit-code-taxonomy.md) the code set, but nothing says how a typed error becomes a process status. It matters here because the wrapper must also reproduce a child's status including signal death, and because `std::process::exit` skips destructors — and the log's non-blocking file sink is flushed by one.

## Considered Options

- Call `std::process::exit` at the failure site.
- Adopt the `sysexits` crate for the `EX_*` constants, handling the rest separately.
- One hand-rolled enum owning every code, converted to `ExitCode` at `main`.

## Decision Outcome

Chosen option: one hand-rolled enum, converted once.

`main` returns `std::process::ExitCode`. Every wrapper failure travels as a value to the entry point, is rendered there, and is converted through a single `From` implementation — which is what a test can assert the whole matrix against in one place.

The `sysexits` crate is rejected because it cannot represent the codes this wrapper owns outside the convention: `1`, `126`, `127`, and a passed-through child status anywhere in `0..=255`. Adopting it would split ownership of the code set in two, and the half it could not hold is the half most easily got wrong.

Signal reproduction is the one sanctioned exception. Re-raising a signal on the wrapper does not return, so it cannot be expressed as an `ExitCode`; it runs at the entry point after the same cleanup, and nowhere else.

## Consequences

- Good: one auditable table, and an unmapped variant is a compile error.
- Good: destructors run, so the log sink flushes before the process ends.
- Bad: the `EX_*` constants are maintained locally rather than pulled from a crate.
- Bad: `main` has one path that does not return, which a reader has to be told about.

## Status

Implemented

Enacted by [`src/main.rs`](../../src/main.rs).

Extends [ADR-0008](./ADR-0008-layered-error-architecture.md), which fixed the error types but not their realization.

Amended by [ADR-0080](./ADR-0080-order-the-boundary-as-report-flush-exit.md), which orders the boundary so the report reaches the log before the sink is flushed.
