# XDG storage

Where every artifact lives, who writes it, and what protects it. For the account/profile split, see [session isolation](../explanation/session-isolation.md).

Base resolution, the private state log namespace, the account directories, the composed-settings store and its pair rule, the five security checks, and the lock-free atomic write are implemented. The credential artifacts, the lock scope, and the last-used marker remain normative design.

## Base directories

| Symbol | Variable          | Default when unset or empty | Holds                                                                                |
| ------ | ----------------- | --------------------------- | ------------------------------------------------------------------------------------ |
| Config | `XDG_CONFIG_HOME` | `$HOME/.config`             | User-authored configuration. Read-only at runtime.                                   |
| State  | `XDG_STATE_HOME`  | `$HOME/.local/state`        | Durable program-written state that survives reboot and is not trivially recreatable. |
| Data   | `XDG_DATA_HOME`   | `$HOME/.local/share`        | Durable program-written data portable between machines.                              |
| Cache  | `XDG_CACHE_HOME`  | `$HOME/.cache`              | Anything safe to delete at any moment.                                               |

Every path is namespaced under `claude-session` inside its base.

A relative XDG value is invalid and treated as unset, with a debug diagnostic. The specification requires it: "All paths set in these environment variables must be absolute. If an implementation encounters a relative path in any of these variables it should consider the path invalid and ignore it." Resolving one against the working directory would put a user's durable state in a different tree on every invocation, which is the same reason a relative `child_bin` is rejected ([process runtime](./process-runtime.md#child-resolution)). An empty value is the unset case, per the same specification's per-variable defaults.

The `0700` on wrapper-managed directories is the specification's own default rather than a wrapper invention: "If, when attempting to write a file, the destination directory is non-existent an attempt should be made to create it with permission `0700`."

`XDG_RUNTIME_DIR` is not used. A lock lives beside the file it guards, so it is reachable wherever that file is ([ADR-0060](../decisions/ADR-0060-lock-the-writes-that-are-not-derivable.md)), and the one base with no portable default is also the one base with nothing to put in it. Durable state never falls back to it or to a shared temporary directory.

## Artifact table

Every artifact has one writer.

| Artifact                 | Base   | Path within base                                | Writer                                               | Mode                           | Lifetime                              |
| ------------------------ | ------ | ----------------------------------------------- | ---------------------------------------------------- | ------------------------------ | ------------------------------------- |
| Wrapper configuration    | Config | `config.toml`                                   | User                                                 | `0644`                         | Until changed                         |
| Project configuration    | none   | `.claude-session.toml` at a repository root     | User                                                 | `0644`                         | Until changed                         |
| Settings pieces          | Config | `settings/<piece>.json`                         | User                                                 | `0644`                         | Until changed                         |
| Profiles                 | Config | `profiles/<profile>.yaml`                       | User                                                 | `0644`                         | Until changed                         |
| Account directory        | State  | `accounts/<account>/`                           | Account subsystem                                    | `0700`                         | Until account removal                 |
| Auth-mode metadata       | State  | `accounts/<account>/auth-mode.json`             | Account subsystem                                    | `0600`                         | Until mode replacement                |
| Local OAuth token        | State  | `accounts/<account>/oauth-token`                | Account subsystem                                    | `0600`                         | Token mode; until rotation or removal |
| Native account config    | State  | `accounts/<account>/config/`                    | Child, after account subsystem creates the directory | `0700`                         | Until account removal                 |
| Native saved login       | State  | `accounts/<account>/config/.credentials.json`   | Child only                                           | Child-managed; expected `0600` | Until child logout or account removal |
| Composed settings        | State  | `composed/profile-<name>-<digest>.json`         | Composition subsystem                                | `0600`                         | Permanent                             |
| Composition provenance   | State  | `composed/profile-<name>-<digest>.compose.json` | Composition subsystem                                | `0600`                         | Permanent                             |
| Last-used account marker | State  | `state/last-account`                            | Account subsystem                                    | `0600`                         | Until selection changes               |
| Write lock               | State  | `.<scope>.lock` beside the files it guards      | Whichever subsystem owns the scope                   | `0600`                         | Permanent; never deleted              |
| Log file                 | State  | `claude-session.log`                            | Logging subsystem                                    | `0600`                         | Rotated                               |

The project configuration file is the one artifact with no XDG base: it lives in the user's repository because that is what makes it per-repository, and it is listed here so the table stays the whole inventory. [Configuration](./configuration.md#project-file-discovery) owns how it is found and what it may set.

The child may create other files and directories below `config/`; it owns their names, contents, modes, and lifecycle.

Credentials are state, not data or cache: they are durable, machine-specific, and unsafe to lose silently. Generated settings are state because removing them during a run changes child behavior.

## Identifiers

| Property        | Rule                            |
| --------------- | ------------------------------- |
| Character set   | `[a-z0-9_-]` only               |
| First character | Lowercase ASCII letter or digit |
| Maximum length  | 32 bytes                        |

These rules apply to account identifiers and to profile names, both of which the wrapper turns into path components. A user-supplied value that fails them exits `Usage`, naming the value and the layer that supplied it; nothing is ever truncated or rewritten.

## Composed settings entries

Each entry is a pure function of its inputs, so its name is computed from them ([ADR-0064](../decisions/ADR-0064-key-composed-settings-by-profile-and-input-digest.md)).

| Part      | Value                                                        |
| --------- | ------------------------------------------------------------ |
| Prefix    | `profile-`, literal                                          |
| Name      | The profile name, under [the identifier rules](#identifiers) |
| Separator | `-`                                                          |
| Digest    | The first 12 lowercase hex characters of the input digest    |
| Suffix    | `.json` for the settings, `.compose.json` for its provenance |

The input digest is SHA-256 over a versioned, unambiguously framed preimage: the literal domain tag `claude-session-composed-v1`, then the profile name, the profile file's resolved absolute path and the SHA-256 of its bytes, then for every piece in profile order its resolved absolute path and the SHA-256 of its bytes. Every field is length-prefixed, so no field value can imitate a field boundary. Paths are hashed as raw OS bytes, since a path is a byte string. The array-strategy table is a field of the profile file, so the profile's own content digest covers it.

The prefix is the field's byte length as eight bytes, big-endian and unsigned, written before every field including the fixed-width digests. Eight bytes because a length on this target is 64-bit and a narrower prefix would need an overflow rule; big-endian because a digest preimage is a wire format. There is no piece count: the prefixes already make the concatenation injective, and a count would be a second thing to keep in agreement. The width is stated because a preimage that cannot be reproduced from its specification is not specified.

Twelve hex characters name the entry; the full digest is recorded in the provenance sidecar. Before an existing entry is reused, that recorded digest is compared against the one just computed — the inputs were read to compute the key, so the comparison costs nothing. A match reuses the entry. A mismatch is [`DataFormat`](./exit-codes.md#wrapper-matrix): the entry is neither opened nor overwritten. That is what makes "two profiles never share settings" a check rather than a probability, and it is why twelve characters is a naming choice rather than a safety margin.

A complete entry is never rewritten. Generation checks whether the settings path exists. If it does, and the sidecar agrees, both files are already correct by construction and the run composes nothing. If neither exists, the wrapper composes, writes the settings by [the atomic sequence](#the-sequence), then writes the provenance the same way. If exactly one member of the pair exists the entry is incomplete and nothing about it can be verified — a settings file without its provenance carries no digest to compare, so adopting it would turn the guarantee above back into a probability. The wrapper composes and writes both members, replacing the survivor with one this run's inputs produced. Two concurrent runs of one profile compute identical bytes, so a lost update is invisible — which is why neither file takes a lock.

## Filesystem security

Checks run on every invocation. What they defend against is recorded in [ADR-0061](../decisions/ADR-0061-protect-storage-from-accidental-local-drift.md): accident — permission drift, a restored backup under the wrong owner, a sync tool that replaced a path with a link — and not a process running as this user, which can read the credential without racing anything.

A wrapper-managed component begins at the `claude-session` namespace directory inside an XDG base. Ancestors supplied by the operating system or the user — `$HOME`, `.config`, `.local/state` — are outside this policy and are never checked or corrected.

| Check                 | Applied to                                                             | On failure               | Reported by                 |
| --------------------- | ---------------------------------------------------------------------- | ------------------------ | --------------------------- |
| Not a symbolic link   | Every wrapper-managed path component                                   | Refuse with `Permission` | `storage-paths-no-symlinks` |
| Owned by current user | Every wrapper-managed path component                                   | Refuse with `Permission` | `storage-paths-owned`       |
| Expected file type    | Every wrapper-managed path                                             | Refuse with `Permission` | `storage-paths-typed`       |
| Mode `0700`           | Wrapper-managed directories                                            | Correct, then proceed    | `storage-directory-modes`   |
| Mode `0600`           | Wrapper-owned secret, metadata, settings, provenance, and marker files | Correct, then proceed    | `storage-secret-modes`      |

Each condition is reported by exactly one [catalog check](./doctor.md#the-catalog), which is what lets a guard and `doctor` describe one problem in one wording ([ADR-0018](../decisions/ADR-0018-one-probe-set-with-stable-check-ids.md)).

### How a path is validated

Immediately before using a wrapper-managed path, the wrapper validates each existing component with metadata operations that do not follow symbolic links, then performs the ordinary path-based operation. A validation result is never cached across operations, because the check is only meaningful against the state the operation will meet.

A wrapper-owned secret that is read is opened once, validated again from that open handle, and read from the same handle. Where the wrapper holds a descriptor it also corrects the mode through it, since a path-based `chmod(2)` dereferences a symbolic link and the symlink-safe form is out of reach — `AT_SYMLINK_NOFOLLOW` on `fchmodat(2)` needs glibc 2.32 and Linux 6.5.

The wrapper does not confine traversal through an `openat(2)` descriptor walk. These checks detect accidental drift and foreign artifacts; they are not a boundary against a process running as this user, which [ADR-0061](../decisions/ADR-0061-protect-storage-from-accidental-local-drift.md) places out of scope. This is the reasoning [ADR-0056](../decisions/ADR-0056-classify-a-failed-spawn-by-its-cause.md) used to reject `fexecve`, applied to the same shape of race.

Managed-directory creation is idempotent, and a directory the wrapper creates is requested `0700` rather than created permissive and then narrowed, so no invocation opens a window in which the component is readable by anyone else. Because `mkdir(2)` applies the process umask to the requested mode, the wrapper settles the mode it just asked for on the component it just created. That only ever widens back toward `0700`, which is why it is not the create-then-correct sequence the rule above rules out.

The wrapper validates the child-owned `.credentials.json` path before relying on its presence, but never changes its mode, rewrites it, or follows it to read credential content.

## Atomic writes

Two hazards, two mechanisms, and neither substitutes for the other ([ADR-0060](../decisions/ADR-0060-lock-the-writes-that-are-not-derivable.md)):

| Hazard      | Symptom                                   | Mechanism     |
| ----------- | ----------------------------------------- | ------------- |
| Torn read   | A reader parses half-old, half-new bytes  | Atomic rename |
| Lost update | A complete but wrong file survives a race | Advisory lock |

Every wrapper-owned file whose partial content would be misread is written by atomic rename: `auth-mode.json` and `oauth-token`, composed settings and composition provenance, and the last-used marker. The child-owned `.credentials.json` is excluded.

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
| 8    | `fsync` the directory                      | The name change is on the disk; step 5 alone does not survive power loss                      |
| 9    | Release                                    | Or exit, which releases it just as completely                                                 |

Steps 1–3 and 9 belong to the [lock scopes](#lock-scopes) below and to nothing else. A lock-free write — composed settings, composition provenance, the last-used marker — performs steps 4 through 8 and no others: there is no scope to acquire, so there is nothing to release.

Steps 5 and 8 are the two points at which the wrapper promises the bytes have reached the disk, and they promise different things: without step 8 a crash can resurrect the old file, or leave a zero-length one at the final name.

The temporary is created with `O_CREAT | O_EXCL`. Its name makes an abandoned one recognizable to [the sweep](#cleanup-and-recovery), and two processes cannot share a process id, so an existing file of that name is an orphan by construction and is removed and recreated once.

### Lock scopes

A lock exists only where a write is not a function of the files it reads, or where two files carry one invariant:

| Scope                                  | Guards                             | Because                                                                        |
| -------------------------------------- | ---------------------------------- | ------------------------------------------------------------------------------ |
| `accounts/<account>/.credentials.lock` | `oauth-token` and `auth-mode.json` | Each login mints a new secret, so a reordered rename can persist a revoked one |

Within that scope the pair is written in one order: `oauth-token` first, then `auth-mode.json`, whose rename commits the rotation ([ADR-0067](../decisions/ADR-0067-commit-a-token-rotation-with-the-metadata-rename.md)). A new token is verified before the lock is taken, so the lock is never held across a child spawn, and the sequence gains no step: verification is not a write.

Composed settings, composition provenance, the last-used marker, and every other wrapper-owned write take no lock. Each is recomputed from its inputs, or is a selection where the most recent write is the right answer. Composed settings and their provenance carry one invariant, and it is expressed in the name: both files are named by the same input digest, so provenance can never describe settings other than the ones beside it.

The lock file is never deleted while its scope exists. Unlinking it lets one holder destroy the file another is about to lock. A permanent empty file is the design, and because it carries no claim, a kill leaves nothing for the next run to break. [`account remove`](./accounts.md#removal) is the one exception, because it destroys the scope itself: it holds the lock, deletes the tree with the lock inside it, and a racer blocked on acquisition wakes on an unlinked inode and fails its rename with `Io` ([ADR-0069](../decisions/ADR-0069-destroy-the-credential-lock-with-its-scope.md)).

`account remove` is also the scope's second writer. It takes the lock before deleting anything, which is what stops a concurrent `account login` writing into a tree being removed. A launch takes no lock — it only reads — so removal excludes no running child and does not look for one.

Acquisition blocks, up to a deadline; past it the run exits [`LockBusy`](./exit-codes.md#wrapper-matrix).

### What this does not promise

Two `account login` runs against one account still end with one token on disk. The lock decides which — the last issued rather than an arbitrary one — and stock `claude` has the same race in its own credential store, so this is not a failure mode the wrapper adds ([ADR-0058](../decisions/ADR-0058-behave-as-stock-claude-by-default.md)).

## Cleanup and recovery

A normal exit removes nothing. Every artifact in the table outlives the run that wrote it by design: configuration is the user's, the account tree and the composed-settings store are the point of the program, and the log is rotated rather than deleted. The one file a run creates without intending to keep is an atomic-write temporary, and that is consumed by its own rename rather than by a cleanup step. The [launch](./process-runtime.md#the-exec) therefore deletes nothing on its way out, and that is the contract rather than an omission.

A kill leaves exactly two things, and neither can fail the next run ([ADR-0058](../decisions/ADR-0058-behave-as-stock-claude-by-default.md)):

| Left behind                           | Why it is harmless                                                                                 |
| ------------------------------------- | -------------------------------------------------------------------------------------------------- |
| An orphaned `.<final-name>.<pid>.tmp` | The final path still holds the previous complete file, since the rename either happened or did not |
| The unflushed tail of the log         | The log is a diagnostic record, and no wrapper behaviour reads it back                             |

A held lock is not on that list. The kernel drops it when the holder's descriptors close, which happens on every death including `SIGKILL`, so a lock is never inherited by the next run as a refusal.

There is no half-written durable state to repair. A reader sees the old complete file or the new one, never a partial one, which is the property the atomic rename is there to buy.

The sweep removes an orphaned temporary from any wrapper-managed directory the invocation already walks for [its security checks](#filesystem-security). A temporary whose embedded process id belongs to a live process is left alone, so a concurrent writer's rename can never be broken by a sweep; the cost is that a temporary from a previous boot whose id has since been reused lingers, which nothing depends on. Lock files are never swept.

Composed settings entries are permanent. Each is immutable and named by its inputs, so one accumulates only when a profile or a piece actually changes — a growth curve set by how often the user edits configuration, not by how many terminals they open. Nothing earns an age policy, a prune verb, or a liveness check at that rate ([ADR-0051](../decisions/ADR-0051-let-every-surface-element-discriminate.md)); removing a store the user no longer wants is `rm`. The orphan-temporary sweep above is the only thing the wrapper deletes unbidden.

`account remove` removes the local account tree and nothing else. Under the [credential lock](#lock-scopes), it unlinks `auth-mode.json` first — an account without its mode metadata is not an account, so that unlink is the commit, inverting [the rotation order](#lock-scopes) — then `oauth-token`, then the child-owned `config/` and every other artifact beneath the account directory, then `.credentials.lock` and the directory itself. `state/last-account` is unlinked only when it names the removed account.

Everything else survives, and the list is exhaustive because silence is what makes users delete by hand: composed settings and their provenance, which are keyed by profile and input digest and carry no account component ([ADR-0064](../decisions/ADR-0064-key-composed-settings-by-profile-and-input-digest.md)); the log file; the whole config base, which is user-authored and never wrapper-written — a configuration layer naming the removed account produces a warning on standard error naming the file and the key, and no edit; every other account; the `accounts/` directory itself, even when the last account goes.

It stops local use but does not claim to revoke a token upstream. A failed first [`account login`](./accounts.md#logging-in) removes the account directory that same run created. Both are deletions a user asked for, which is what separates them from a sweep.

## Diagnostics

`doctor` reports resolved base directories, environment-versus-default provenance, the selected account path and the resolved composed-settings entry path, and the five [security checks](#filesystem-security) named in the table above. Child credential content is never inspected or emitted.

Config-base artifacts, the log file, and lock files carry no security check. Configuration is user-authored and `0644` by design, so there is no unsafe state to report; a log or a lock that cannot be opened must not stop a passthrough run ([ADR-0058](../decisions/ADR-0058-behave-as-stock-claude-by-default.md)), and lock contention already has [`LockBusy`](./exit-codes.md#wrapper-matrix).
