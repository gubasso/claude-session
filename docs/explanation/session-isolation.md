# Session isolation

`claude-session` separates two scopes. An account supplies durable native identity, authentication, configuration, projects, history, and trust state. A profile supplies the composed settings the child is launched with, and their provenance. This page explains that split.

Exact paths, modes, and writers are in [XDG storage](../reference/xdg-storage.md). Launch injection is in [the wrapper model](./wrapper-model.md).

## What a session is

A wrapper session is an account plus a profile:

- The account selects one shared child `config/` directory and one stored authentication mode.
- The profile selects one composed `settings.json` and its provenance sidecar.

Every run of one account uses the same `CLAUDE_CONFIG_DIR`. The child exclusively owns its saved login and other native state there. The wrapper passes the profile's composed document through the native `--settings` flag. The two selections are independent: an account can be selected without a profile, and a profile without an account.

Sessions are not conversations. Runs of one account intentionally share the child's projects, history, onboarding, and trust state. A caller needing conversation separation uses the child's own session identifier.

## Why the profile is the key

The composed document is a pure function of the profile, its ordered pieces, and their contents. Nothing about the terminal, the working directory, or the project enters it. Keying it by any of those is the error of keying a build artifact by who ran the build: two runs that should share an artifact get two, and two runs that should not share one get one.

Two profiles launched from one terminal are the case that decides it. They must never meet, and under an input-addressed key they cannot: each names its own entry, and an entry is written once and never rewritten. The converse holds too — identical inputs from different terminals, or from different accounts, name one entry, which is correct because the bytes are identical. A wrapper of this same shape was observed keying composed child configuration by terminal instead, so that a second profile in one terminal overwrote the first and unlinked files a live child depended on; that is the failure this split exists to make unrepresentable.

Whole-root-per-profile isolation was rejected: it would split the login, history, and trust that one account exists to share. Exact names and the write rule are in [XDG storage](../reference/xdg-storage.md#composed-settings-entries); why, in [ADR-0064](../decisions/ADR-0064-key-composed-settings-by-profile-and-input-digest.md).

## Why both scopes are state

Account config and composed settings are durable program-written state that may survive reboot. They are not:

- user-authored Configuration;
- safely disposable Cache.

The Runtime base is unused: the wrapper opens no socket, and its [write locks](../reference/xdg-storage.md#lock-scopes) live beside the files they guard rather than in a separate tree ([ADR-0060](../decisions/ADR-0060-lock-the-writes-that-are-not-derivable.md)). Durable state is never relocated to it or to a shared temporary directory.

## One writer per artifact

- The child writes account `config/`, including its saved login, projects, history, and trust state.
- The account subsystem writes `auth-mode.json`, any local OAuth token, and the last-used marker.
- The composition subsystem writes composed settings and their provenance.

The wrapper never reads, copies, fingerprints, or synchronizes the child credential. Wrapper-owned files use atomic write-then-rename; directory creation is idempotent.

## Security posture

Wrapper-managed directories are private, non-symlink, and current-user-owned. Checks run on every invocation rather than relying on creation-time state. Wrapper-owned secrets and metadata are private and atomically replaced.

What those checks defend against is accident — drift, a restored backup, a sync tool — and not a process running as this user, which needs no race to read a credential it already has access to ([ADR-0061](../decisions/ADR-0061-protect-storage-from-accidental-local-drift.md)). That boundary is why validation is a non-following metadata pass rather than a confined traversal.

The child-owned credential is validated only as a path when presence matters. The wrapper does not read, chmod, rewrite, or emit it. Cleanup never follows symbolic links.

## Cleanup

Composed settings entries are immutable and permanent; the wrapper ships no pruning. Automatic cleanup removes nothing but an orphaned atomic temporary, and only explicit account removal deletes an account tree — which leaves composed settings alone, because they are not account state.

## Further reading

- [XDG Base Directory Specification](https://specifications.freedesktop.org/basedir/latest/)
- [`directories`](https://docs.rs/directories/)
- [`xdg-ninja`](https://github.com/b3nj5m1n/xdg-ninja)
