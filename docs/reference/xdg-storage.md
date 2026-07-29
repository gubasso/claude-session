# XDG storage

Where every artifact lives, who writes it, and what protects it. For the reasoning behind the session directory's placement, see [session isolation](../explanation/session-isolation.md).

This describes normative design. The crate is pre-implementation.

## Base directories

| Symbol  | Variable          | Default when unset or empty | Holds                                                                                  |
| ------- | ----------------- | --------------------------- | -------------------------------------------------------------------------------------- |
| Config  | `XDG_CONFIG_HOME` | `$HOME/.config`             | User-authored configuration. Read-only at runtime.                                     |
| State   | `XDG_STATE_HOME`  | `$HOME/.local/state`        | Durable program-written state that survives a reboot and is not trivially recreatable. |
| Data    | `XDG_DATA_HOME`   | `$HOME/.local/share`        | Durable program-written data that is portable between machines.                        |
| Cache   | `XDG_CACHE_HOME`  | `$HOME/.cache`              | Anything safe to delete at any moment.                                                 |
| Runtime | `XDG_RUNTIME_DIR` | **No portable default**     | Ephemeral coordination: locks, sockets, process identifiers. Cleared on logout.        |

Every path is namespaced under a `claude-session` directory inside its base.

Two rules about the variables themselves:

**A relative value is invalid and is ignored.** The specification requires absolute paths; an implementation encountering a relative one treats the variable as unset and uses the default. This program does the same, and says so at debug verbosity rather than failing.

**Runtime has no fallback.** The specification defines no default for the runtime directory, and it is genuinely absent in containers, under `cron`, and over some remote logins. When it is missing the wrapper **degrades explicitly**: it reports that runtime-dependent behaviour is unavailable and continues without it. It does not fall back to the state directory, and it never falls back to a shared temporary directory.

That last prohibition is load-bearing. A fallback chain ending in a world-writable temporary directory would put credentials somewhere any local user can reach, and a chain that redirects durable state into a directory cleared at logout would present as a mysterious logout rather than as an error.

## Artifact table

Every artifact has exactly one writer. Reads are unrestricted; writes are owned.

The one artifact the wrapper hands off is the session credential copy: the account subsystem creates it by seeding, and from then on the child owns its contents. That handoff is the whole point of the per-session copy — see [ADR-0011](../decisions/0011-isolate-credentials-by-seed-and-session.md) — and it is why a refreshed token inside a session does not propagate back to the account's seed.

| Artifact                 | Base    | Path within the base                                             | Writer                                                     | Mode   | Lifetime                         |
| ------------------------ | ------- | ---------------------------------------------------------------- | ---------------------------------------------------------- | ------ | -------------------------------- |
| Wrapper configuration    | Config  | `config.toml`                                                    | The user                                                   | `0644` | Until the user changes it        |
| Settings pieces          | Config  | `settings/<piece>.json`                                          | The user                                                   | `0644` | Until the user changes it        |
| Profile manifests        | Config  | `manifests/<profile>.yaml`                                       | The user                                                   | `0644` | Until the user changes it        |
| Account directory        | State   | `accounts/<account>/`                                            | Account subsystem                                          | `0700` | Until the account is removed     |
| Credential seed          | State   | `accounts/<account>/credentials.json`                            | Account subsystem                                          | `0600` | Until re-authenticated           |
| Session directory        | State   | `accounts/<account>/groups/<group>/`                             | Session subsystem                                          | `0700` | Until pruned as stale            |
| Session credentials      | State   | `accounts/<account>/groups/<group>/` (child-managed name)        | Account subsystem seeds it; the child writes it thereafter | `0600` | Session lifetime                 |
| Generated settings       | State   | `accounts/<account>/groups/<group>/settings.json`                | Composition subsystem                                      | `0600` | Regenerated when stale           |
| Composition provenance   | State   | `accounts/<account>/groups/<group>/.claude-session-compose.json` | Composition subsystem                                      | `0600` | Alongside the generated settings |
| Session metadata         | State   | `accounts/<account>/groups/<group>/session-meta.json`            | Session subsystem                                          | `0600` | Session lifetime                 |
| Last-used account marker | State   | `state/last-account`                                             | Account subsystem                                          | `0600` | Until the account changes        |
| Log file                 | State   | `claude-session.log`                                             | Logging                                                    | `0600` | Rotated                          |
| Sync locks               | Runtime | `locks/<name>.lock`                                              | Whoever takes the lock                                     | `0600` | Process lifetime                 |

Three placements deserve their reasons:

**Credentials are state, not data and not cache.** They are durable and machine-specific, so state rather than data. They are catastrophic to lose silently, so never cache — a cache-clearing tool that logs the user out is a bug that presents as a mystery.

**The generated settings file is state, not cache**, even though it is regenerable. It is regenerable only while its inputs exist and are readable; and a session whose settings vanish mid-run behaves erratically rather than failing cleanly.

**Locks are runtime, not state.** A lock file surviving a reboot is a stale lock, which is strictly worse than no lock.

## Group identifiers

The group identifier names the session directory, so it must be filesystem-safe and stable.

| Property        | Rule                                                         |
| --------------- | ------------------------------------------------------------ |
| Character set   | `[a-z0-9_-]` only                                            |
| First character | A lowercase ASCII letter or a digit                          |
| Maximum length  | 32 bytes                                                     |
| Derivation      | See [session isolation](../explanation/session-isolation.md) |

A derived value that fails validation is rejected, not silently truncated or rewritten. Truncation invites collision, which in this program means two terminals sharing a session directory.

Account identifiers use the same rules.

## Filesystem security

These checks run on **every** invocation, not only at creation. A directory created before a `umask` change, or restored from a careless backup, is a real leak.

| Check                     | Applied to                          | On failure               |
| ------------------------- | ----------------------------------- | ------------------------ |
| Not a symbolic link       | Every directory in the session path | Refuse with `Permission` |
| Owned by the current user | Every directory in the session path | Refuse with `Permission` |
| Is a directory            | Every directory in the session path | Refuse with `Permission` |
| Mode is `0700`            | Every managed directory             | Correct it, then proceed |
| Mode is `0600`            | Every credential file               | Correct it, then proceed |

The link check uses a metadata call that does **not** follow symbolic links, and it is applied to each path component in turn rather than only to the leaf. Following a symlink into a directory another user controls is the classic way a program's write becomes an attacker's write.

Directory creation is idempotent: creating a directory that already exists and passes its checks succeeds. Two invocations from the same pane can race, and neither should fail.

## Atomic writes

Every file whose partial contents would be misread is written by creating a temporary file in the **same directory**, writing and flushing it, setting its mode, and renaming it over the target. Same directory because rename is atomic only within a filesystem.

A reader therefore sees either the complete old file or the complete new one, never a truncated one. This applies to credential seeds, generated settings, provenance sidecars, session metadata, and the last-account marker.

## Cleanup

Stale session directories are pruned by age, conservatively:

- Only directories under a `groups/` parent are candidates. Account directories are never pruned automatically.
- Symbolic links are never followed out of the tree being pruned.
- A directory that might still be in use is left alone. Leaving a stale directory on disk costs bytes; deleting a live session costs the user their work.
- Pruning is opt-in and reported.

## Diagnostics

`claude-session doctor` reports the resolved value of every base directory, which of them came from an environment variable versus a default, the resolved session directory, and the result of each security check — without mutating anything. See [logging and output](./logging-and-output.md).

## Further reading

- [XDG Base Directory Specification](https://specifications.freedesktop.org/basedir/latest/)
- [`directories`](https://docs.rs/directories/)
- [`rustix`](https://docs.rs/rustix/)
- [`xdg-ninja`](https://github.com/b3nj5m1n/xdg-ninja)
