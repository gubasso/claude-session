# XDG storage

Where every artifact lives, who writes it, and what protects it. For the account/group split, see [session isolation](../explanation/session-isolation.md).

This describes normative design. The crate is pre-implementation.

## Base directories

| Symbol | Variable          | Default when unset or empty | Holds                                                                                |
| ------ | ----------------- | --------------------------- | ------------------------------------------------------------------------------------ |
| Config | `XDG_CONFIG_HOME` | `$HOME/.config`             | User-authored configuration. Read-only at runtime.                                   |
| State  | `XDG_STATE_HOME`  | `$HOME/.local/state`        | Durable program-written state that survives reboot and is not trivially recreatable. |
| Data   | `XDG_DATA_HOME`   | `$HOME/.local/share`        | Durable program-written data portable between machines.                              |
| Cache  | `XDG_CACHE_HOME`  | `$HOME/.cache`              | Anything safe to delete at any moment.                                               |

Every path is namespaced under `claude-session` inside its base.

A relative XDG value is invalid and treated as unset, with a debug diagnostic. The specification requires it: _"All paths set in these environment variables must be absolute. If an implementation encounters a relative path in any of these variables it should consider the path invalid and ignore it."_ Resolving one against the working directory would put a user's durable state in a different tree on every invocation, which is the same reason a relative `child_bin` is rejected ([process runtime](./process-runtime.md#child-resolution)). An empty value is the unset case, per the same specification's per-variable defaults.

The `0700` on wrapper-managed directories is the specification's own default rather than a wrapper invention: _"If, when attempting to write a file, the destination directory is non-existent an attempt should be made to create it with permission `0700`."_

`XDG_RUNTIME_DIR` is not used. A lock lives beside the file it guards, so it is reachable wherever that file is ([ADR-0060](../decisions/ADR-0060-lock-the-writes-that-are-not-derivable.md)), and the one base with no portable default is also the one base with nothing to put in it. Durable state never falls back to it or to a shared temporary directory.

## Artifact table

Every artifact has one writer.

| Artifact                 | Base   | Path within base                                                 | Writer                                               | Mode                           | Lifetime                              |
| ------------------------ | ------ | ---------------------------------------------------------------- | ---------------------------------------------------- | ------------------------------ | ------------------------------------- |
| Wrapper configuration    | Config | `config.toml`                                                    | User                                                 | `0644`                         | Until changed                         |
| Settings pieces          | Config | `settings/<piece>.json`                                          | User                                                 | `0644`                         | Until changed                         |
| Profiles                 | Config | `profiles/<profile>.yaml`                                        | User                                                 | `0644`                         | Until changed                         |
| Account directory        | State  | `accounts/<account>/`                                            | Account subsystem                                    | `0700`                         | Until account removal                 |
| Auth-mode metadata       | State  | `accounts/<account>/auth-mode.json`                              | Account subsystem                                    | `0600`                         | Until mode replacement                |
| Local OAuth token        | State  | `accounts/<account>/oauth-token`                                 | Account subsystem                                    | `0600`                         | Token mode; until rotation or removal |
| Native account config    | State  | `accounts/<account>/config/`                                     | Child, after account subsystem creates the directory | `0700`                         | Until account removal                 |
| Native saved login       | State  | `accounts/<account>/config/.credentials.json` on Linux/Windows   | Child only                                           | Child-managed; expected `0600` | Until child logout or account removal |
| Group directory          | State  | `accounts/<account>/groups/<group>/`                             | Session subsystem                                    | `0700`                         | Until stale pruning                   |
| Generated settings       | State  | `accounts/<account>/groups/<group>/settings.json`                | Composition subsystem                                | `0600`                         | Regenerated when stale                |
| Composition provenance   | State  | `accounts/<account>/groups/<group>/.claude-session-compose.json` | Composition subsystem                                | `0600`                         | With generated settings               |
| Session metadata         | State  | `accounts/<account>/groups/<group>/session-meta.json`            | Session subsystem                                    | `0600`                         | Group lifetime                        |
| Last-used account marker | State  | `state/last-account`                                             | Account subsystem                                    | `0600`                         | Until selection changes               |
| Write lock               | State  | `.<scope>.lock` beside the files it guards                       | Whichever subsystem owns the scope                   | `0600`                         | Permanent; never deleted              |
| Log file                 | State  | `claude-session.log`                                             | Logging subsystem                                    | `0600`                         | Rotated                               |

The child may create other files and directories below `config/`; it owns their names, contents, modes, and lifecycle. On macOS, the child stores ordinary login material in Keychain rather than the relocated credential path; see [accounts](./accounts.md#platform-boundary).

Credentials are state, not data or cache: they are durable, machine-specific, and unsafe to lose silently. Generated settings are state because removing them during a run changes child behavior.

## Group identifiers

| Property        | Rule                            |
| --------------- | ------------------------------- |
| Character set   | `[a-z0-9_-]` only               |
| First character | Lowercase ASCII letter or digit |
| Maximum length  | 32 bytes                        |

Account identifiers use the same rules. Why the group is derived this way is in [session isolation](../explanation/session-isolation.md#deriving-the-group-without-knowing-multiplexers) and [ADR-0062](../decisions/ADR-0062-derive-the-group-from-the-controlling-terminal.md).

### The derivation ladder

First hit wins.

| # | Input                                                 | Identifier                    |
| - | ----------------------------------------------------- | ----------------------------- |
| 0 | `--session <id>`                                      | The value, unchanged          |
| 1 | `CLAUDE_SESSION_GROUP`                                | The value, unchanged          |
| 2 | `ttyname_r(3)` on a descriptor opened from `/dev/tty` | `<disc>-pts-3`, `<disc>-tty1` |
| 3 | `getsid(0)` and that process's start time             | `<disc>-s-<sid>-<start>`      |
| 4 | 64 bits of operating-system randomness                | `<disc>-r-<hex>`              |

Rung 2 opens `/dev/tty`; nothing consults `isatty(0)`, so a piped invocation still derives from the terminal it has ([ADR-0053](../decisions/ADR-0053-read-a-confirmation-from-the-controlling-terminal.md)). Its identifier is the device path with `/dev/` stripped and `/` replaced by `-`. Rung 3 reads `/proc/<sid>/stat` field 22 on Linux; the start time is what makes a reused process id a different identity. Rung 4 emits one warning; the rungs above it emit none.

`<disc>` is the host discriminator from [ADR-0063](../decisions/ADR-0063-claim-a-group-by-its-derivation-fingerprint.md): six hex characters of a keyed hash of `/etc/machine-id`, or of the hostname when that file is absent or empty. It is always present. The machine identity itself is never stored or printed.

### Rejection is not fallthrough

The two rules differ by who supplied the value, because the useful answer differs:

| Source                     | A value failing the rules      |
| -------------------------- | ------------------------------ |
| Rung 0 or 1, user-supplied | Exits `Usage`                  |
| Rung 2 or 3, derived       | Falls through to the next rung |

Neither is ever truncated or rewritten. A user who typed a bad identifier wants to know; an exotic device name is the wrapper's problem to route around, and failing the run over one would make a working terminal unusable.

### Claiming a group

A group's `session-meta.json` records the full derivation fingerprint — the rung, the terminal path, its `st_rdev` and `(st_dev, st_ino)`, the session leader and start time, and the pid and mount namespace ids. Before a group is used:

| State               | Action                                                    |
| ------------------- | --------------------------------------------------------- |
| No directory        | Create it and claim it                                    |
| Fingerprint matches | Reuse it                                                  |
| Fingerprint differs | Never open its settings; derive a suffixed group and warn |

This is what makes "two separate sessions never share a group" a check rather than a probability, and it is reported by [`session-group-claim`](./logging-and-output.md#the-catalog). The [`.settings.lock`](#lock-scopes) does not cover it: a lock serializes two writers to one path, and cannot tell that they are unrelated sessions.

## Filesystem security

Checks run on every invocation. What they defend against is recorded in [ADR-0061](../decisions/ADR-0061-protect-storage-from-accidental-local-drift.md): accident — permission drift, a restored backup under the wrong owner, a sync tool that replaced a path with a link — and not a process running as this user, which can read the credential without racing anything.

A **wrapper-managed component** begins at the `claude-session` namespace directory inside an XDG base. Ancestors supplied by the operating system or the user — `$HOME`, `.config`, `.local/state` — are outside this policy and are never checked or corrected.

| Check                 | Applied to                                                             | On failure               | Reported by                 |
| --------------------- | ---------------------------------------------------------------------- | ------------------------ | --------------------------- |
| Not a symbolic link   | Every wrapper-managed path component                                   | Refuse with `Permission` | `storage-paths-no-symlinks` |
| Owned by current user | Every wrapper-managed path component                                   | Refuse with `Permission` | `storage-paths-owned`       |
| Expected file type    | Every wrapper-managed path                                             | Refuse with `Permission` | `storage-paths-typed`       |
| Mode `0700`           | Wrapper-managed directories                                            | Correct, then proceed    | `storage-directory-modes`   |
| Mode `0600`           | Wrapper-owned secret, metadata, settings, provenance, and marker files | Correct, then proceed    | `storage-secret-modes`      |

Each condition is reported by exactly one [catalog check](./logging-and-output.md#the-catalog), which is what lets a guard and `doctor` describe one problem in one wording ([ADR-0018](../decisions/ADR-0018-one-probe-set-with-stable-check-ids.md)).

### How a path is validated

Immediately before using a wrapper-managed path, the wrapper validates each existing component with metadata operations that **do not follow symbolic links**, then performs the ordinary path-based operation. A validation result is never cached across operations, because the check is only meaningful against the state the operation will meet.

A wrapper-owned secret that is read is opened once, validated again from that open handle, and read from the same handle. Where the wrapper holds a descriptor it also corrects the mode through it, since a path-based `chmod(2)` dereferences a symbolic link and the symlink-safe form is out of reach — `AT_SYMLINK_NOFOLLOW` on `fchmodat(2)` needs glibc 2.32 and Linux 6.5.

The wrapper does **not** confine traversal through an `openat(2)` descriptor walk. These checks detect accidental drift and foreign artifacts; they are not a boundary against a process running as this user, which [ADR-0061](../decisions/ADR-0061-protect-storage-from-accidental-local-drift.md) places out of scope. This is the reasoning [ADR-0056](../decisions/ADR-0056-classify-a-failed-spawn-by-its-cause.md) used to reject `fexecve`, applied to the same shape of race.

Managed-directory creation is idempotent, and a directory the wrapper creates is created `0700` rather than created and then corrected.

The wrapper validates the child-owned `.credentials.json` path before relying on its presence, but never changes its mode, rewrites it, or follows it to read credential content.

## Atomic writes

Two hazards, two mechanisms, and neither substitutes for the other ([ADR-0060](../decisions/ADR-0060-lock-the-writes-that-are-not-derivable.md)):

| Hazard      | Symptom                                   | Mechanism     |
| ----------- | ----------------------------------------- | ------------- |
| Torn read   | A reader parses half-old, half-new bytes  | Atomic rename |
| Lost update | A complete but wrong file survives a race | Advisory lock |

**Every** wrapper-owned file whose partial content would be misread is written by atomic rename: `auth-mode.json` and `oauth-token`, generated settings and composition provenance, session metadata, and the last-used marker. The child-owned `.credentials.json` is excluded.

### The sequence

| Step | Operation                                  | Why this step exists                                                                          |
| ---- | ------------------------------------------ | --------------------------------------------------------------------------------------------- |
| 1    | Acquire the in-process mutex for the scope | A lock is per open file description, so it cannot exclude a second thread of the same process |
| 2    | Open `.<scope>.lock`, creating if absent   | The sentinel is a lock handle, not a claim                                                    |
| 3    | Take the exclusive lock                    | Excludes other processes until this one exits or releases                                     |
| 4    | Write `.<final-name>.<pid>.tmp`            | Same directory, so the rename stays within one filesystem                                     |
| 5    | `fsync` the temporary                      | The bytes are on the disk                                                                     |
| 6    | Set the mode on the temporary              | The final name is never briefly world-readable                                                |
| 7    | Rename onto the final name                 | The swap a reader can never observe half of                                                   |
| 8    | `fsync` the directory                      | The **name change** is on the disk; step 5 alone does not survive power loss                  |
| 9    | Release                                    | Or exit, which releases it just as completely                                                 |

Steps 5 and 8 are the two points at which the wrapper promises the bytes have reached the disk, and they promise different things: without step 8 a crash can resurrect the old file, or leave a zero-length one at the final name.

The temporary is created with `O_CREAT | O_EXCL`. Its name makes an abandoned one recognizable to [the sweep](#cleanup-and-recovery), and two processes cannot share a process id, so an existing file of that name is an orphan by construction and is removed and recreated once.

### Lock scopes

A lock exists only where a write is **not** a function of the files it reads, or where two files carry one invariant:

| Scope                                  | Guards                                             | Because                                                                        |
| -------------------------------------- | -------------------------------------------------- | ------------------------------------------------------------------------------ |
| `accounts/<account>/.credentials.lock` | `oauth-token` and `auth-mode.json`                 | Each login mints a new secret, so a reordered rename can persist a revoked one |
| `.../groups/<group>/.settings.lock`    | `settings.json` and `.claude-session-compose.json` | Provenance claiming freshness over stale settings is a state nothing corrects  |

Session metadata and the last-used marker take no lock. Both are recomputed from their inputs, or are a selection where the most recent write is the right answer.

**The lock file is never deleted.** Unlinking it lets one holder destroy the file another is about to lock. A permanent empty file is the design, and because it carries no claim, a kill leaves nothing for the next run to break.

Acquisition blocks, up to a deadline; past it the run exits [`TempFail`](./exit-codes.md#wrapper-matrix).

### What this does not promise

Two `account login` runs against one account still end with one token on disk. The lock decides **which** — the last issued rather than an arbitrary one — and stock `claude` has the same race in its own credential store, so this is not a failure mode the wrapper adds ([ADR-0058](../decisions/ADR-0058-behave-as-stock-claude-by-default.md)).

## Cleanup and recovery

**A normal exit removes nothing.** Every artifact in the table outlives the run that wrote it by design: configuration is the user's, the account and group trees are the point of the program, and the log is rotated rather than deleted. The one file a run creates without intending to keep is an atomic-write temporary, and that is consumed by its own rename rather than by a cleanup step. [Post-flight](./process-runtime.md#post-flight) therefore deletes nothing, and that is the contract rather than an omission.

**A kill leaves exactly two things**, and neither can fail the next run ([ADR-0058](../decisions/ADR-0058-behave-as-stock-claude-by-default.md)):

| Left behind                           | Why it is harmless                                                                                 |
| ------------------------------------- | -------------------------------------------------------------------------------------------------- |
| An orphaned `.<final-name>.<pid>.tmp` | The final path still holds the previous complete file, since the rename either happened or did not |
| The unflushed tail of the log         | The log is a diagnostic record, and no wrapper behaviour reads it back                             |

A held lock is not on that list. The kernel drops it when the holder's descriptors close, which happens on every death including `SIGKILL`, so a lock is never inherited by the next run as a refusal.

There is no half-written durable state to repair. A reader sees the old complete file or the new one, never a partial one, which is the property the atomic rename is there to buy.

**The sweep** removes an orphaned temporary from any wrapper-managed directory the invocation already walks for [its security checks](#filesystem-security). A temporary whose embedded process id belongs to a live process is left alone, so a concurrent writer's rename can never be broken by a sweep; the cost is that a temporary from a previous boot whose id has since been reused lingers, which nothing depends on. Lock files are never swept.

Stale-group pruning is conservative:

- only directories below `groups/` are candidates;
- symbolic links are never followed;
- a possibly active group is retained, decided by age;
- pruning is opt-in and reported;
- account-wide `config/`, mode metadata, and token storage are never pruned.

`account remove` removes the local account tree, including child-owned config and all groups. It stops local use but does not claim to revoke a token upstream.

## Diagnostics

`doctor` reports resolved base directories, environment-versus-default provenance, selected account and group paths, and the five [security checks](#filesystem-security) named in the table above. Child credential content is never inspected or emitted.

Config-base artifacts, the log file, and lock files carry no security check. Configuration is user-authored and `0644` by design, so there is no unsafe state to report; a log or a lock that cannot be opened must not stop a passthrough run ([ADR-0058](../decisions/ADR-0058-behave-as-stock-claude-by-default.md)), and lock contention already has [`LockBusy`](./exit-codes.md#wrapper-matrix).
