# ADR-0080: Order the boundary as report, flush, then exit

## Context and Problem Statement

[ADR-0035](./ADR-0035-convert-the-typed-error-to-a-code-once.md) put the conversion at `main` but left the order of the three things that happen there unstated. The entry point flushed the log sink before rendering the failure, so the error record reached a joined worker and was dropped: the file exists for a user who ran with no flags, and that is exactly when it lost the record. Interleaving the spine with the report also kept `?` out of the entry point, since a function returning `ExitCode` cannot use it.

## Considered Options

- Leave the entry point flat and move the flush after the report.
- Let the fallible program own the guard and drop it on the way out.
- Give the entry point the guard, delegate the work, and fix the order.

## Decision Outcome

Chosen option: the entry point owns the guard and delegates.

`main` classifies argv, resolves the state namespace, installs logging, and calls a fallible program that reads no process global. The boundary then runs in one order: report the failure while the sink is alive, flush by dropping the guard, then return the code or exec the child.

Reporting before the flush is what makes the record survive. Exec'ing after it keeps the guarantee [ADR-0035](./ADR-0035-convert-the-typed-error-to-a-code-once.md) already made, and the flush cannot follow an image replacement that never returns.

Two failures stay unreported to the log by construction: a wrapper cannot know where to write before argv is classified and the namespace resolved, and a failed exec meets a sink that has already been flushed.

## Consequences

- Good: the last record survives, and the fallible program regains `?`.
- Good: the argument grammar is exercisable without a process.
- Bad: two orderings share one boundary, which a reader has to be told about.
- Bad: the pre-logging failure and a failed exec are diagnosable only from standard error.

## Status

Implemented

Enacted by [`src/main.rs`](../../src/main.rs).

Amends [ADR-0035](./ADR-0035-convert-the-typed-error-to-a-code-once.md), which fixed the conversion but not its order. Amended by [ADR-0084](./ADR-0084-exec-the-child-instead-of-supervising-it.md), which made the last step an exec.
