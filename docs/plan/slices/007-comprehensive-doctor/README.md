# 007 — Comprehensive doctor

## Goal

Land the `doctor` verb: the shared report engine plus every catalog check whose subject already exists.

## Appetite

2 implementation sessions.

## Core

Every catalog check runs independently and the documented ordered exit rule holds in every renderer.

## In scope

- Shared human and JSON rendering over the public check catalog.
- `--list` catalog output and `--strict` warning promotion.
- Non-aborting traversal, deterministic catalog order, and the ordered hard-failure exit rule.
- Consistent remediation, skipped-check reasons, and summary counts.
- The host probes whose subject exists: base directories, the runtime directory, wrapper configuration parsing, child resolution, child executability, and the child version floor against the documented minimum.
- Rendering for the storage and settings checks already implemented at their guard call sites, over the one probe set those guards read.
- The composed child report and the single soft check its exit status contributes.

## Out of scope

- The `account-registry-readable` and `credentials-usable` entries, whose subject the account rung creates.
- The hard login-mode refusal that reuses the version-floor entry; slice 005 owns it.
- Checks absent from the catalog or aborting the report after one failure.
- Parallel probe execution.

## Governed by

- [AGENTS.md](../../../../AGENTS.md)
- [Doctor](../../../reference/doctor.md)
- [Process runtime](../../../reference/process-runtime.md)
- [XDG storage](../../../reference/xdg-storage.md)
- [Configuration](../../../reference/configuration.md)
- [CLI surface](../../../reference/cli-surface.md)
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
- When the resolved child reports a version below the documented minimum or one that does not parse, the probe shall warn without failing the run.
- Where a guard and this report describe one condition, both shall emit the catalog remediation without paraphrase.

## Rabbit holes

- Pulling feature probes back into this slice; escape: the owning feature rung appends and implements each subject-specific catalog entry.
- Implementing a probe whose subject does not exist yet; escape: a check that could only ever report `skipped` is a placeholder, so leave its row to the rung that creates the subject.
- Parallel probe execution; escape: preserve deterministic catalog order.

## Done when

Targeted text, JSON, list, strict, skip, summary, ordered-exit, non-aborting, version-floor, and guard-parity `assert_cmd` checks pass under `cargo nextest`, and `just hooks` is green.

## Revisions

None.
