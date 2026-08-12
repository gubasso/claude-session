# ADR-0088: Model nothing the child already owns

## Context and Problem Statement

The wrapper kept a list of the child's recognized top-level settings keys and warned on anything outside it. The list held fourteen names against the child's own reference of sixty-five or more, so the common case was warning about a valid setting rather than catching a typo. Every future release of the child widens that gap, and the list has no mechanism that can close it.

## Considered Options

- Grow the list from the child's published reference and revalidate it each release.
- Keep the list, and promote the warning to a rejection once the schema is measured.
- Keep no list, and validate only what composition itself requires.

## Decision Outcome

Chosen option: keep no list. A warning that fires more often on correct input than on incorrect input teaches users to ignore it, which also costs the real defects it was built to catch. Tracking someone else's evolving schema is work the child already does, with better information and its own error messages, so a wrapper-side copy can only lag it.

The boundary is ownership, not caution. The wrapper parses and validates the profile document, the strategy table and its applicability, each piece being well-formed JSON with an object root, cross-piece type conflicts, and the composed document's well-formedness — each a property of the composition rather than of what a key means. Beyond that it generates the document consistently and hands it over, which is [ADR-0002](./ADR-0002-verbatim-argv-passthrough.md) applied to settings.

## Consequences

- Good: a valid new child setting works the day it ships, with no wrapper release.
- Good: one deleted module, one deleted report field, and no perishable table to revalidate.
- Bad: a misspelled settings key is silent here and surfaces only as the child ignoring it.

## Status

Accepted

Amends [ADR-0010](./ADR-0010-compose-native-settings-from-declared-layers.md), whose permissive-with-warnings validation this replaces with no validation at all outside the composition's own structure. Amends [ADR-0049](./ADR-0049-collapse-config-inspection-into-one-verb.md) by removing the unknown-key warnings from what `config` reports. Enacted in `src/services/storage/entry.rs` and `docs/reference/configuration.md#validation`.
