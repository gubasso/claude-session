# 007 — Comprehensive doctor

## Goal

Finish the shared `doctor` report engine over catalog entries supplied by the feature rungs that own each check.

## Appetite

1 implementation session.

## Core

Every catalog check runs independently and the documented ordered exit rule holds in every renderer.

## In scope

- Shared human and JSON rendering over the public check catalog.
- `--list` catalog output and `--strict` warning promotion.
- Non-aborting traversal, deterministic catalog order, and the ordered hard-failure exit rule.
- Consistent remediation, skipped-check reasons, and summary counts.

## Out of scope

- Feature-specific catalog entries and probes, which land as tail work in the rung that creates their subject.
- Checks absent from the catalog or aborting the report after one failure.
- Parallel probe execution.

## Governed by

- [AGENTS.md](../../../../AGENTS.md)
- [Doctor](../../../reference/doctor.md)
- [Exit codes](../../../reference/exit-codes.md)
- [Logging and output](../../../reference/logging-and-output.md)
- [Presentation](../../../reference/presentation.md)
- [Testing and quality](../../../reference/testing-and-quality.md)

## Acceptance

- When one subsystem check fails, the wrapper shall continue through every remaining catalog entry.
- When no hard check fails, the wrapper shall exit zero unless `--strict` promotes a warning to one.
- If hard checks fail, then the wrapper shall return the first failure's code in catalog order.
- When a feature is not configured, its eligible soft check shall report `skipped` without changing status.
- When `doctor --list`, human output, or JSON output is requested, the wrapper shall render the same ordered public catalog and consistent remediation facts.

## Rabbit holes

- Pulling feature probes back into this slice; escape: the owning feature rung appends and implements each subject-specific catalog entry.
- Parallel probe execution; escape: preserve deterministic catalog order.

## Done when

Targeted text, JSON, list, strict, skip, summary, ordered-exit, and non-aborting `assert_cmd` checks pass under `cargo nextest`, and `just hooks` is green.

## Revisions

None.
