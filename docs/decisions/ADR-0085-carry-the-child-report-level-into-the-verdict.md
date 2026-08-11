# ADR-0085: Carry the child report level into the verdict

## Context and Problem Statement

[ADR-0045](./ADR-0045-compose-doctor-with-the-child-report.md) made the child's exit status one probe in the wrapper's report, with any nonzero value a warning. The child is the application this wrapper exists to run, so that downgrade lets `claude-session doctor` exit `0` and report a healthy summary while `claude doctor` says the thing being wrapped is broken. Only `--strict` surfaced it, and the counts never explained the promotion.

## Considered Options

- Carry the child's level unchanged into a composed verdict over both reports.
- Keep the child's exit as a soft probe, as [ADR-0045](./ADR-0045-compose-doctor-with-the-child-report.md) first wrote it.
- Append the child's status to the check catalog as a stable id.

## Decision Outcome

Chosen option: carry the level — a composed run has two answers and must state both faithfully, and the child's answer is not the wrapper's to soften.

A composed run reports three levels, each with its own status and code: the wrapper's catalog, the child's report, and the verdict over both. A failed child report makes the verdict fail without `--strict`. The verdict exits `Unavailable`, keeping the status inside the wrapper matrix that [ADR-0068](./ADR-0068-spawn-the-child-as-a-subroutine.md) requires of a subroutine verb, while the child's own code is reported as data beside it. A wrapper hard failure outranks the child, because it usually explains it. `--strict` is unchanged and now concerns warnings alone.

The catalog was rejected because it does not generalize: the next composed verb under [ADR-0079](./ADR-0079-compose-every-overlapping-surface-with-the-child.md) has no catalog to append to, while three levels fit any composition.

## Consequences

- Good: the reported counts always explain the exit, because each level owns one.
- Good: a broken child can no longer be reported as a healthy run.
- Bad: `doctor` exits nonzero for a condition the wrapper cannot repair.
- Bad: human mode must capture the child's output to place the verdict before it.

## Status

Implemented

Implemented by `Verdict::fold` in [the check catalog](../../src/domain/checks.rs), rendered by [the doctor report](../../src/ui/doctor.rs). Amends [ADR-0045](./ADR-0045-compose-doctor-with-the-child-report.md), whose composition and passthrough-of-output rules are otherwise unchanged.
