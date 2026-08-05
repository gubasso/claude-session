# ADR-0025: Share one native login per account

## Context and Problem Statement

[ADR-0011](./ADR-0011-isolate-credentials-by-seed-and-session.md) copied a saved login per session to avoid concurrent refresh conflicts. Child version 2.1.211 introduced coordination only among processes sharing one saved login; separate copies bypass that lock and recreate the rotate-and-revoke failure.

## Considered Options

- Share one durable native configuration directory per account.
- Copy credentials in and out.
- Link or bind-mount one credential into group directories.
- Serialize every child behind a wrapper-wide lock.

## Decision Outcome

Chosen option: one account-wide native configuration directory — every run of an account receives its durable `config/` as `CLAUDE_CONFIG_DIR`; groups retain only composed settings, provenance, and session metadata.

The wrapper owns selection of the account directory. The child exclusively owns `config/.credentials.json`: the wrapper never reads, copies, writes, refreshes, fingerprints, or synchronizes it. Wrapper `account login`, passthrough `auth login`, and an in-TUI `/login` structurally reach the same child-owned location, without a wrapper handshake, lock, cache, or write-back. Exact TUI behavior remains externally tracked.

## Consequences

- Good: concurrent runs participate in the child's refresh coordination.
- Good: wrapper credential and trust-state synchronization disappear.
- Bad: project history, trust, onboarding, plugins, and other native state are shared by runs of one account.
- Bad: per-group settings require the native-flag mechanism in [ADR-0028](./ADR-0028-pass-composed-settings-with-the-native-flag.md).
- Bad: login mode requires child version 2.1.211, enforced at launch by [ADR-0031](./ADR-0031-enforce-the-child-refresh-lock-version-floor.md).

Copy-in/copy-out loses crash safety; symlinks and bind mounts have replacement or portability hazards; a wrapper-wide lock would serialize valid concurrent work.

## Status

Accepted

Supersedes [ADR-0011](./ADR-0011-isolate-credentials-by-seed-and-session.md).

Amended by [ADR-0064](./ADR-0064-key-composed-settings-by-profile-and-input-digest.md) — the shared account `config/` this record establishes is unchanged; what a group retained is now profile-owned composed settings, stored outside the account tree.
