# ADR-0049: Collapse config inspection into one verb

## Context and Problem Statement

`config` carried six subcommands — `view`, `path`, `schema`, `compose`, `validate`, `status` — that between them answered one question: what is my configuration, where did it come from, and is it sound? Splitting one question across six verbs makes the user pick the right slice before they can ask, and four of the six had no present caller, which [ADR-0048](./ADR-0048-build-for-a-present-need.md) rejects.

## Considered Options

- Keep the six subcommands, each answering its slice.
- Fold inspection into `doctor`, leaving no `config` verb.
- One bare `config` verb that resolves, validates, and reports in a single output.

## Decision Outcome

Chosen option: one bare `config` verb. It reports the resolved wrapper configuration with per-key provenance, the active profile and its resolved pieces, generated-settings freshness, and every structural defect and unknown-key warning — one command, one answer, `--json` for machines.

`config` is an assertion verb, alongside `doctor` in [exit codes](../reference/exit-codes.md#exit-regimes-by-verb): it validates, so a defect it cannot function with exits with that defect's code rather than reporting failure at `0`.

`doctor` stays a pure checker and is not a configuration renderer. The two do not duplicate logic: under [ADR-0018](./ADR-0018-one-probe-set-with-stable-check-ids.md) there is one probe catalog, `config` runs its config-scoped subset, `doctor` runs the whole of it, and both quote the same remediation verbatim.

`config schema` is redundant with the generated committed schema ([ADR-0013](./ADR-0013-generate-config-examples-from-types.md)). `profile status` is redundant with `config --profile <name>`; discovering accepted profiles remains a separate need.

## Consequences

- Good: the specification and test surface shrink by five verbs before any of them was written.
- Bad: `config` now exits non-zero on a defect, so a script that only wants the values must tolerate that or read `--json`.
- Bad: a single output is denser than six narrow ones.

## Status

Accepted

Amended by [ADR-0051](./ADR-0051-let-every-surface-element-discriminate.md) to collapse `profile list`, by [ADR-0064](./ADR-0064-key-composed-settings-by-profile-and-input-digest.md) to report entry existence instead of mtime freshness, and by [ADR-0088](./ADR-0088-model-nothing-the-child-already-owns.md), which drops the unknown-key warnings this record's chosen option lists from what the verb reports. The one-verb decision remains.
