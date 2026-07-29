# Session isolation

Native `claude` keeps its credentials, conversation state, and configuration in a single directory shared by every instance the user runs. Two terminals are two views of one state. `claude-session` gives each terminal its own. This page explains what a session is, how one is identified, and why the directory holding it belongs where it does.

The path table — which artifact lives under which base directory, with which mode and which writer — is in [XDG storage](../reference/xdg-storage.md). The environment-injection mechanism is described in [the wrapper model](./wrapper-model.md).

## What a session is

A **session** is one isolated configuration directory, plus the metadata describing how it was derived. When the wrapper spawns the child, it points the child's configuration-directory variable at that path. From the child's perspective nothing is unusual: it reads and writes its own configuration directory as it always does. It simply is not the same directory another terminal is using.

A session belongs to an **account** and to a **group**. The account says whose credentials the session uses; the group says which terminal it belongs to. The directory layout nests accordingly — accounts at the top, groups beneath — so that a session's full identity is readable from its path.

Sessions are not conversations. The child owns conversation state inside the directory; the wrapper owns the directory itself. That boundary matters: the wrapper creates, secures, seeds, and eventually prunes the directory, and does not otherwise interpret what the child puts in it.

## Deriving the group, without knowing about multiplexers

The interesting question is: two shells are running: are they the same session or different ones?

The tempting answer is to ask the terminal multiplexer. Every one of them exports a pane identifier, and reading it is a few lines of code. This project deliberately does **not** do that.

Multiplexer sniffing fails in three ways. It does not work in a plain terminal with no multiplexer at all. It does not work in a multiplexer nobody thought to add. And it grows: each new tool is another branch, another variable name, another special case, in a program whose whole design principle is knowing as little as possible about its environment.

The general answer is the **controlling terminal**. Every interactive tab, split, and pane owns a distinct pseudo-terminal, and it does so as a property of how terminals work rather than as a property of any particular multiplexer. Keying on the controlling terminal gives per-pane isolation everywhere, for free, with no tool-specific knowledge. It is the same answer in tmux, in a bare console, and in whatever ships next year.

Derivation is a priority chain, tried in order, first hit wins:

1. An explicit flag. The user said which session they want; that ends the discussion.
2. An explicit environment variable, for scripted and container use where a flag is awkward.
3. The controlling terminal, sanitized into a filesystem-safe identifier. This is the normal path.
4. The parent process, combined with its start time, for a non-interactive parent with no controlling terminal. The start time is what keeps a recycled process id from colliding with a stale session.
5. The current process id, with a **visible warning**, as a last resort.

The last rung deserves its warning. Keying on the process id means the session is unique to this invocation and will never be found again — every run gets a fresh directory. That is correct behaviour in a headless pipeline, and it is a symptom worth surfacing anywhere else, because the user probably expected continuity they are not getting.

The chain never crashes. A wrapper that refuses to run because it could not identify a terminal has failed at its one job.

The exact identifier syntax and length limits are in [XDG storage](../reference/xdg-storage.md).

## The container edge

The controlling-terminal key is unique within one kernel's view of the world. If a state directory is bind-mounted into several containers, two containers can each hold a pane with the same terminal name, and the two would land in the same session directory.

This is a real collision but a narrow one, and the fix stays in keeping with the design: when — and only when — that scenario is supported, the group key is namespaced with **one** neutral host or container discriminator. Neutral means it identifies the machine or container without identifying any particular tool: a machine identity file, a hostname, a control-group-derived identifier. It is off by default, because paying its cost universally to fix a scenario most users never hit is how a small program stops being small.

## Why the session directory is state

Placing the session directory correctly is a decision that is expensive to reverse, because users' credentials end up in it.

The session directory holds **durable state**: credentials copied in from the account seed, the child's accumulated configuration, and the wrapper's own session metadata. Durable state that the user cannot trivially recreate belongs under the state base directory. It is not:

- **Configuration.** Configuration is authored by the user and read-only at runtime. The session directory is written by the program on every run. Putting program-written state in the configuration tree makes the user's dotfile repository churn and makes "reset my config" ambiguous.
- **Cache.** Cache is by definition safe to delete. A credential is not. The moment a session directory can be cleaned up by a cache-clearing tool, the program has a data-loss bug, and it will present as a mysterious logout.
- **Runtime.** The runtime directory is for ephemeral coordination — locks, sockets, process identifiers — and is cleared when the user logs out. It is the right home for a lock file and the wrong home for a credential.

There is a specific trap here. The runtime directory is optional: the specification defines no portable fallback for it, and it is genuinely absent in containers, in cron, and over some remote logins. It is therefore tempting to write a fallback chain that lands durable state in runtime when state is unavailable, or in a temporary directory when both are. Both are wrong. A fallback that can lose credentials is worse than a clear error, and a world-writable temporary directory is the wrong home for a secret under any circumstances. When the runtime directory is absent the wrapper degrades **explicitly** — it says so and continues without whatever needed it — and durable state never moves.

## One writer per artifact

Every file the wrapper manages has exactly one component that writes it. Reads are unrestricted; writes are owned.

This is what makes concurrency reasoning tractable in a program where several terminals are alive at once against the same account. Two sessions sharing an account read the same credential seed; only the account subsystem ever writes it. Two sessions each write their own metadata; neither touches the other's. Where a genuinely shared artifact must be updated — syncing project-trust state back to the seed after a session ends — the write is serialized by a lock and merges conservatively rather than overwriting, because the other session's newer state is not this session's to discard.

Directory creation is idempotent and atomic, because the same pane can plausibly launch two invocations at once. Every file write that must not be observed half-finished goes through a write-to-temporary-then-rename, so a reader sees either the old file or the new one and never a truncated one.

## Security posture

Session directories hold credentials, so the filesystem checks are not decoration:

- Directories are created private to the user, and the mode is **enforced** on every run rather than assumed from creation. A directory created before a `umask` change, or restored from a careless backup, is a real leak.
- Before trusting a path, the wrapper checks that it is not a symbolic link and that it is owned by the current user. Following a symlink into a directory someone else controls is the classic way a privileged write becomes an attacker's write.
- Credential files are private to the user and written atomically. Cleanup never follows symbolic links out of the tree it is pruning.

These checks are cheap and they are run every time. The alternative — trusting a directory because the program created it once — assumes nothing else on the system ever touches the filesystem.

## Cleanup

Session directories accumulate: every pane that ever ran the wrapper leaves one. Pruning is deliberately conservative, age-based, and never surprising. A session directory that might still be in use is left alone, because the cost of deleting a live session is far higher than the cost of leaving a stale directory on disk.

## Further reading

- [XDG Base Directory Specification](https://specifications.freedesktop.org/basedir/latest/)
- [`directories`](https://docs.rs/directories/)
- [`xdg-ninja`, a home-directory conformance checker](https://github.com/b3nj5m1n/xdg-ninja)
