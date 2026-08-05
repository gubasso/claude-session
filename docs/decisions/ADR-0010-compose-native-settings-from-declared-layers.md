# ADR-0010: Compose the child's settings from declared layers

## Context and Problem Statement

Each isolated session needs its own settings file for the child. Users vary settings by context — work versus personal, permissive versus restricted — without wanting several near-identical copies whose shared parts drift apart.

## Considered Options

- One settings file per profile, copied and edited by the user.
- A templating language over a single settings file.
- Partial settings pieces, composed in an order declared by a per-profile manifest.

## Decision Outcome

Chosen option: pieces composed by a manifest — copies drift, and a templating language means inventing a second configuration syntax on top of the child's own.

The user authors read-only JSON pieces, each a fragment in the child's format, and a YAML manifest per profile listing pieces in order. The wrapper folds them left to right and writes the result atomically into the session directory, never editing the child's files in place.

Three details are load-bearing. Merge is deterministic — same inputs, byte-identical output — or the freshness check and diffs are both useless. Arrays replace by default, with concatenation and merge-by-key opt-in per key, because a global array strategy is wrong for half of any real settings file. And a provenance sidecar records which piece set each leaf key.

Validation is deliberately asymmetric: strict about the structure the wrapper owns, permissive about unknown keys in the child's schema, which evolves independently. See [configuration](../reference/configuration.md).

## Consequences

- Good: provenance makes composition debuggable in one command rather than by bisecting files.
- Good: the child sees an ordinary settings file and needs no awareness of the wrapper.
- Bad: the freshness check must consider every referenced piece, not just the manifest; checking only the manifest leaves stale settings after a piece edit, a bug that is hard to diagnose.
- Bad: permissive validation means a typo in a child settings key is a warning, not an error.

## Status

Accepted

Amended by [ADR-0028](./ADR-0028-pass-composed-settings-with-the-native-flag.md) for native-flag delivery, [ADR-0064](./ADR-0064-key-composed-settings-by-profile-and-input-digest.md) for input-addressed storage, and [ADR-0050](./ADR-0050-name-the-profile-surface-once.md) for the profile name. The composition model remains unchanged.
