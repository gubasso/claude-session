# ADR-0047: Let a user settings flag override the group layer

## Context and Problem Statement

[ADR-0028](./ADR-0028-pass-composed-settings-with-the-native-flag.md) prepends `--settings <group settings path>` and left duplicate-flag behaviour explicitly unverified. Measured against `claude` 2.1.220 on 2026-07-31, the child keeps only the **last** occurrence: an earlier settings file is not merged, not validated, and not read at all. Since the wrapper's pair is a prefix, a user's own `--settings` always wins, silently discarding the group's composed layer.

## Considered Options

- Accept the child's precedence: a user-supplied `--settings` is a total, intentional override.
- Append the wrapper's pair after the user suffix so the wrapper wins.
- Parse the user's argv and drop or merge a duplicate settings flag.

## Decision Outcome

Chosen option: **accept the child's precedence** — the losing layer is the wrapper's, the winning one is the user's explicit instruction, and that is the safe direction for a tool whose first contract is not interfering.

Appending the pair after the user suffix was rejected because tokens after `--` are the child's positional territory, so a suffix would corrupt exactly the invocations the sentinel exists to protect. Parsing to deduplicate is the coupling [ADR-0002](./ADR-0002-verbatim-argv-passthrough.md) and [ADR-0028](./ADR-0028-pass-composed-settings-with-the-native-flag.md) both reject.

The behaviour is documented where a user meets it, in [the child argument vector](../reference/process-runtime.md#child-argument-vector) and [configuration](../reference/configuration.md): passing `--settings` yourself replaces the group's composed document rather than adding to it. Users who want both compose them into one file and pass that.

## Consequences

- Good: no argv inspection, no deduplication, and the passthrough contract is untouched.
- Good: precedence runs the way users expect — the flag they typed wins.
- Bad: a user who passes `--settings` silently loses profile composition, and the wrapper cannot warn without parsing.
- Bad: the behaviour is the child's to change, so it is registered as a perishable fact rather than a guarantee.

## Status

Accepted

Amends [ADR-0028](./ADR-0028-pass-composed-settings-with-the-native-flag.md).
