# ADR-0018: One probe set with stable check identifiers

## Context and Problem Statement

`doctor` has a documented check catalog, but nothing says whether the checks a command runs before doing work are the same checks. Two probe sets drift, and the failure mode is the worst one a diagnostic can have: `doctor` reports healthy while a command fails its own guard. The checks also have no names, so nothing — a script, an error message, a document — can refer to one.

## Considered Options

- **`doctor` only**, with each command writing its preconditions inline.
- **One shared catalog** of anonymous checks.
- **One catalog with stable identifiers**, consumed by `doctor` and by every command guard.

## Decision Outcome

Chosen option: **one catalog with stable identifiers**.

Every probe carries a stable kebab-case id, a scope, and a severity. `doctor` runs the whole catalog; a command guard runs the subset it requires and, on failure, emits that check's remediation **verbatim** rather than a paraphrase. Adding a prerequisite means adding a catalog entry, never bolting a check onto one call site.

Identifiers are **public API**, on the same footing as `err.kind` in [ADR-0005](./ADR-0005-exit-code-taxonomy.md): scripts match them, so renaming one is a breaking change and the set is append-only in spirit. Each id maps to the `err.kind` that a failure of it exits with, which is what lets a guard's failure and `doctor`'s report describe one problem in one wording.

Severity is the only waiver lever. A check that could legitimately be ignored is soft by definition, so there is no per-invocation ignore flag and a hard check stays an unconditional guarantee.

The catalog itself lives in [logging and output](../reference/logging-and-output.md).

## Consequences

- Good: `doctor` and command guards cannot disagree, because there is only one set.
- Good: a user told to run `doctor` sees the same wording their failure gave them.
- Bad: check ids join the compatibility surface that cannot be renamed freely.
- Bad: a guard costs a catalog entry, which is heavier than an inline check.

## Status

Accepted
