# ADR-0064: Key composed settings by profile and input digest

## Context and Problem Statement

The composed `settings.json` was stored under a terminal-derived group. Two profiles launched from one pane — `claude-session --profile work &` then `claude-session --profile review` — derive the same group and write one path; the lock serialises the writes but cannot tell that they are unrelated, so the second run's complete file replaces the first while the first child is still pointed at it. A wrapper of this same shape was observed doing exactly this, additionally unlinking sibling files a live child depended on. Composition has no terminal-derived input: the output is a pure function of the profile, its ordered pieces, and their contents.

## Considered Options

- `accounts/<account>/groups/<group>/settings.json` — the terminal key.
- `composed/profile-<name>/settings.json` — a directory per profile.
- `composed/profile-<name>/<digest>.json` — a digest inside that directory.
- `composed/profile-<name>-<digest>.json` — flat, immutable, input-addressed.
- A fresh temporary directory per run.
- A whole per-profile child root, with `CLAUDE_CONFIG_DIR` per profile.

## Decision Outcome

Chosen option: flat, input-addressed entries under the XDG state `composed/` store, pairing `profile-<name>-<digest>.json` with its `.compose.json` sidecar. [XDG storage](../reference/xdg-storage.md#composed-settings-entries) owns the grammar, digest preimage, and write rule.

The digest covers inputs, not output, because provenance differs when two input sets produce one value.

The store leaves `accounts/` because no input is account-scoped. A per-profile directory retains mid-flight rewrite and freshness. A per-run temporary recomposes and leaks state. A per-profile child root splits the shared login [ADR-0025](./ADR-0025-share-one-native-login-per-account.md) keeps whole.

## Consequences

- Good: two profiles in one terminal can never meet; an entry is written once and never rewritten, so no lock, and existence replaces the mtime freshness check.
- Bad: profile names gain the identifier grammar, because a name becomes a path component.
- Bad: entries accumulate, and this record ships no way to remove them.

## Status

Accepted

Amends [ADR-0006](./ADR-0006-place-files-by-xdg-ownership.md), [ADR-0010](./ADR-0010-compose-native-settings-from-declared-layers.md), [ADR-0018](./ADR-0018-one-probe-set-with-stable-check-ids.md), [ADR-0025](./ADR-0025-share-one-native-login-per-account.md), [ADR-0028](./ADR-0028-pass-composed-settings-with-the-native-flag.md), [ADR-0047](./ADR-0047-let-a-user-settings-flag-override-the-group-layer.md), [ADR-0049](./ADR-0049-collapse-config-inspection-into-one-verb.md), and [ADR-0060](./ADR-0060-lock-the-writes-that-are-not-derivable.md).
