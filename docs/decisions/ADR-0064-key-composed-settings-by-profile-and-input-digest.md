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

Chosen option: **flat, input-addressed entries** under `$XDG_STATE_HOME/claude-session/composed/` — `profile-work-8f2a91c3d40b.json` beside `profile-work-8f2a91c3d40b.compose.json`. The prefix says what the name is for, the name says which profile, the digest says which inputs. The grammar, digest preimage, and write rule are in [XDG storage](../reference/xdg-storage.md#composed-settings-entries).

The digest covers the **inputs**, never the composed output: the sidecar records which piece set each key, which is a function of the inputs, so output-addressing could let two input sets collide on one sidecar path with different content.

The store leaves `accounts/` because no composition input is account-scoped; the child applies its own account `config/settings.json` layer itself. A per-profile directory keeps the lock, the freshness check, and the same-profile mid-flight rewrite. A per-run temporary directory contradicts this project's refusal to put durable state in a shared temporary directory, recomposes every launch, and leaks a directory no later run can attribute. A per-profile child root would split the shared login [ADR-0025](./ADR-0025-share-one-native-login-per-account.md) exists to keep whole.

## Consequences

- Good: two profiles in one terminal can never meet; an entry is written once and never rewritten, so no lock, and existence replaces the mtime freshness check.
- Good: the same inputs name the same entry, which makes composition diagnosable by filename.
- Bad: profile names gain the identifier grammar, because a name becomes a path component.
- Bad: entries accumulate, and this record ships no way to remove them.

## Status

Accepted

Amends [ADR-0006](./ADR-0006-place-files-by-xdg-ownership.md), [ADR-0010](./ADR-0010-compose-native-settings-from-declared-layers.md), [ADR-0018](./ADR-0018-one-probe-set-with-stable-check-ids.md), [ADR-0025](./ADR-0025-share-one-native-login-per-account.md), [ADR-0028](./ADR-0028-pass-composed-settings-with-the-native-flag.md), [ADR-0047](./ADR-0047-let-a-user-settings-flag-override-the-group-layer.md), [ADR-0049](./ADR-0049-collapse-config-inspection-into-one-verb.md), and [ADR-0060](./ADR-0060-lock-the-writes-that-are-not-derivable.md).
