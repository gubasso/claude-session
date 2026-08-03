# Session isolation

`claude-session` separates two scopes. An account supplies durable native identity, authentication, configuration, projects, history, and trust state. A terminal group supplies composed settings, provenance, and session metadata. This page explains that split and how the group is identified.

Exact paths, modes, and writers are in [XDG storage](../reference/xdg-storage.md). Launch injection is in [the wrapper model](./wrapper-model.md).

## What a session is

A wrapper session is an **account plus a group**:

- The account selects one shared child `config/` directory and one stored authentication mode.
- The group selects one per-terminal `settings.json`, composition-provenance sidecar, and metadata document.

Every run of one account uses the same `CLAUDE_CONFIG_DIR`. The child exclusively owns its saved login and other native state there. The wrapper passes the group's settings through the native `--settings` flag.

Sessions are not conversations. Runs of one account intentionally share the child's projects, history, onboarding, and trust state. A caller needing conversation separation uses the child's own session identifier.

## Deriving the group without knowing multiplexers

The wrapper identifies terminal context without recognizing tmux, screen, or any other multiplexer. Tool-specific variables fail in plain terminals, omit future multiplexers, and expand the wrapper's environment grammar.

The controlling terminal is the general key: interactive tabs, splits, and panes have distinct pseudo-terminals regardless of the program that created them. Derivation is first-hit-wins over five rungs ([ADR-0062](../decisions/ADR-0062-derive-the-group-from-the-controlling-terminal.md)); the exact inputs and the identifier they produce are in [XDG storage](../reference/xdg-storage.md#group-identifiers).

Two properties of that ladder are worth understanding rather than looking up.

**It survives detach and reattach.** A multiplexer creates a pane's pseudo-terminal once, in its server, when the pane is spawned. Detaching disconnects a client and leaves that terminal alone, so a run before a detach and a run after reattaching — possibly from a different machine — land in the same group. Everything an emulator or multiplexer exports about a window describes the _client_ instead, which is exactly the thing reattaching changes. That is the concrete reason those variables are rejected, rather than a preference for kernel interfaces.

**It never fails, and the rung it reached is reportable.** The last rung is random, so it is honest: the group is new, will not be rediscovered, and says so. A headless pipeline that reaches the session-leader rung above it is not warned, because there the identity is correct and stable.

## The container edge

A controlling-terminal name is unique only within one kernel view. Several containers that bind-mount one state directory derive identical terminal names, and a group silently shared between two sessions merges their settings and metadata with no error.

Two mechanisms answer that, and [ADR-0063](../decisions/ADR-0063-claim-a-group-by-its-derivation-fingerprint.md) records both. A host discriminator prefixes the identifier, so the ordinary case does not collide; it is always applied and names no container tool. A recorded derivation fingerprint, compared before a group is used, is what makes the guarantee unconditional — including where the discriminator itself is cloned along with a container image. A run that meets a group whose fingerprint is not its own never opens that group's settings.

## Why both scopes are state

Account config and group artifacts are durable program-written state that may survive reboot. They are not:

- user-authored Configuration;
- safely disposable Cache.

The Runtime base is unused: the wrapper opens no socket, and its [write locks](../reference/xdg-storage.md#lock-scopes) live beside the files they guard rather than in a separate tree ([ADR-0060](../decisions/ADR-0060-lock-the-writes-that-are-not-derivable.md)). Durable state is never relocated to it or to a shared temporary directory.

## One writer per artifact

- The child writes account `config/`, including its saved login, projects, history, and trust state.
- The account subsystem writes `auth-mode.json`, any local OAuth token, and the last-used marker.
- The composition subsystem writes group settings and provenance.
- The session subsystem writes group metadata.

The wrapper never reads, copies, fingerprints, or synchronizes the child credential. Wrapper-owned files use atomic write-then-rename; directory creation is idempotent.

## Security posture

Wrapper-managed directories are private, non-symlink, and current-user-owned. Checks run on every invocation rather than relying on creation-time state. Wrapper-owned secrets and metadata are private and atomically replaced.

What those checks defend against is accident — drift, a restored backup, a sync tool — and not a process running as this user, which needs no race to read a credential it already has access to ([ADR-0061](../decisions/ADR-0061-protect-storage-from-accidental-local-drift.md)). That boundary is why validation is a non-following metadata pass rather than a confined traversal.

The child-owned credential is validated only as a path when presence matters. The wrapper does not read, chmod, rewrite, or emit it. Cleanup never follows symbolic links.

## Cleanup

Group directories accumulate and are pruned conservatively by age. A possibly active group is retained. Automatic pruning never removes account-wide config or authentication state; only explicit account removal deletes that tree.

## Further reading

- [XDG Base Directory Specification](https://specifications.freedesktop.org/basedir/latest/)
- [`directories`](https://docs.rs/directories/)
- [`xdg-ninja`](https://github.com/b3nj5m1n/xdg-ninja)
- [`credentials(7)`](https://man7.org/linux/man-pages/man7/credentials.7.html) — sessions, process groups, and the controlling terminal
- [Linux devpts documentation](https://www.kernel.org/doc/html/latest/filesystems/devpts.html) — why a pty index is namespace-local
