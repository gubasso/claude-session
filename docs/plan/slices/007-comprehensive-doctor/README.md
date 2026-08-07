# 007 — Comprehensive doctor

## Goal

Make `doctor` a complete non-aborting health report over all implemented subsystems.

## Appetite

1 implementation session.

## Core

Every catalog check runs independently and the documented ordered exit rule holds.

## In scope

- Human and JSON renderers.
- `--list` catalog output.
- `--strict` warning promotion.
- Consistent remediation text.

## Out of scope

- Checks absent from the catalog.
- Aborting the report after one failure.

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

## Rabbit holes

- Private checks outside the catalog; escape: append public catalog entries through their owner first.
- Parallel probe execution; escape: preserve deterministic catalog order.

## Done when

Targeted text, JSON, list, strict, skip, and non-aborting `assert_cmd` checks pass under `cargo nextest`.

## Revisions

None.
