# ADR-0028: Pass composed settings with the native flag

## Context and Problem Statement

One account must share a durable child configuration directory, while settings composition remains specific to a terminal group. The wrapper needs to deliver both without turning `CLAUDE_CONFIG_DIR` back into a per-group credential boundary or violating [ADR-0002](./ADR-0002-verbatim-argv-passthrough.md).

## Considered Options

- Prepend the child's native `--settings <absolute path>` pair.
- Make `CLAUDE_CONFIG_DIR` per group.
- Rewrite a changing profile into the shared account directory.
- Parse and deduplicate the user's child flags.

## Decision Outcome

Chosen option: prepend the native settings pair — the account directory remains stable while each group supplies its composed settings document as an additional native layer.

The wrapper-added pair precedes the untouched user argument vector. Order, bytes, count, and any `--` sentinel are preserved within that suffix. The wrapper does not parse, deduplicate, reorder, or reject user child flags.

## Consequences

- Good: credentials and other child-owned account state stay shared per account.
- Good: each terminal group keeps its composed settings and provenance.
- Bad: behavior with two `--settings` occurrences is unverified and must be tested and revalidated before implementation. No merge, precedence, or equivalence rule is assumed.
- Bad: the child invocation contains one declared wrapper-added pair in addition to the verbatim user suffix.

See [ADR-0010](./ADR-0010-compose-native-settings-from-declared-layers.md), [configuration](../reference/configuration.md), and [process runtime](../reference/process-runtime.md).

## Status

Accepted

Amended by [ADR-0047](./ADR-0047-let-a-user-settings-flag-override-the-group-layer.md), which discharges the unverified duplicate-flag consequence: the child keeps the last occurrence, so a user-supplied `--settings` replaces the group layer.

Amended by [ADR-0064](./ADR-0064-key-composed-settings-by-profile-and-input-digest.md) — the document this record prepends is keyed by profile and input digest rather than by a terminal group. The prefix mechanism, the untouched user suffix, and the shared account directory are unchanged.
