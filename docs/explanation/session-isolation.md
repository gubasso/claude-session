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

The controlling terminal is the general key: interactive tabs, splits, and panes have distinct pseudo-terminals regardless of the program that created them.

Derivation is first-hit-wins:

1. Explicit wrapper flag.
2. Explicit environment variable.
3. Controlling terminal, sanitized as a filesystem-safe identifier.
4. Parent process plus start time when no controlling terminal exists.
5. Current process id, with a visible warning.

The last rung creates an invocation-specific group that will not be rediscovered. It is valid for a headless pipeline and warning-worthy elsewhere. Derivation never crashes the wrapper. Identifier rules live in [XDG storage](../reference/xdg-storage.md#group-identifiers).

## The container edge

A controlling-terminal name is unique only within one kernel view. If several containers bind-mount the same state directory, identical terminal names can collide.

Support for that scenario may namespace the group with one neutral host or container discriminator: machine identity, hostname, or control-group identity. It remains off by default and does not recognize a specific container tool.

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

The child-owned credential is validated only as a path when presence matters. The wrapper does not read, chmod, rewrite, or emit it. Cleanup never follows symbolic links.

## Cleanup

Group directories accumulate and are pruned conservatively by age. A possibly active group is retained. Automatic pruning never removes account-wide config or authentication state; only explicit account removal deletes that tree.

## Further reading

- [XDG Base Directory Specification](https://specifications.freedesktop.org/basedir/latest/)
- [`directories`](https://docs.rs/directories/)
- [`xdg-ninja`](https://github.com/b3nj5m1n/xdg-ninja)
