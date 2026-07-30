# XDG storage

Where every artifact lives, who writes it, and what protects it. For the account/group split, see [session isolation](../explanation/session-isolation.md).

This describes normative design. The crate is pre-implementation.

## Base directories

| Symbol  | Variable          | Default when unset or empty | Holds                                                                                |
| ------- | ----------------- | --------------------------- | ------------------------------------------------------------------------------------ |
| Config  | `XDG_CONFIG_HOME` | `$HOME/.config`             | User-authored configuration. Read-only at runtime.                                   |
| State   | `XDG_STATE_HOME`  | `$HOME/.local/state`        | Durable program-written state that survives reboot and is not trivially recreatable. |
| Data    | `XDG_DATA_HOME`   | `$HOME/.local/share`        | Durable program-written data portable between machines.                              |
| Cache   | `XDG_CACHE_HOME`  | `$HOME/.cache`              | Anything safe to delete at any moment.                                               |
| Runtime | `XDG_RUNTIME_DIR` | **No portable default**     | Ephemeral locks, sockets, and process identifiers.                                   |

Every path is namespaced under `claude-session` inside its base.

A relative XDG value is invalid and treated as unset, with a debug diagnostic. Runtime has no fallback: if absent, runtime-dependent behavior is reported unavailable. It never falls back to State or a shared temporary directory.

## Artifact table

Every artifact has one writer.

| Artifact                 | Base    | Path within base                                                 | Writer                                               | Mode                           | Lifetime                              |
| ------------------------ | ------- | ---------------------------------------------------------------- | ---------------------------------------------------- | ------------------------------ | ------------------------------------- |
| Wrapper configuration    | Config  | `config.toml`                                                    | User                                                 | `0644`                         | Until changed                         |
| Settings pieces          | Config  | `settings/<piece>.json`                                          | User                                                 | `0644`                         | Until changed                         |
| Profile manifests        | Config  | `manifests/<profile>.yaml`                                       | User                                                 | `0644`                         | Until changed                         |
| Account directory        | State   | `accounts/<account>/`                                            | Account subsystem                                    | `0700`                         | Until account removal                 |
| Auth-mode metadata       | State   | `accounts/<account>/auth-mode.json`                              | Account subsystem                                    | `0600`                         | Until mode replacement                |
| Local OAuth token        | State   | `accounts/<account>/oauth-token`                                 | Account subsystem                                    | `0600`                         | Token mode; until rotation or removal |
| Native account config    | State   | `accounts/<account>/config/`                                     | Child, after account subsystem creates the directory | `0700`                         | Until account removal                 |
| Native saved login       | State   | `accounts/<account>/config/.credentials.json` on Linux/Windows   | Child only                                           | Child-managed; expected `0600` | Until child logout or account removal |
| Group directory          | State   | `accounts/<account>/groups/<group>/`                             | Session subsystem                                    | `0700`                         | Until stale pruning                   |
| Generated settings       | State   | `accounts/<account>/groups/<group>/settings.json`                | Composition subsystem                                | `0600`                         | Regenerated when stale                |
| Composition provenance   | State   | `accounts/<account>/groups/<group>/.claude-session-compose.json` | Composition subsystem                                | `0600`                         | With generated settings               |
| Session metadata         | State   | `accounts/<account>/groups/<group>/session-meta.json`            | Session subsystem                                    | `0600`                         | Group lifetime                        |
| Last-used account marker | State   | `state/last-account`                                             | Account subsystem                                    | `0600`                         | Until selection changes               |
| Log file                 | State   | `claude-session.log`                                             | Logging subsystem                                    | `0600`                         | Rotated                               |
| Sync locks               | Runtime | `locks/<name>.lock`                                              | Lock holder                                          | `0600`                         | Process lifetime                      |

The child may create other files and directories below `config/`; it owns their names, contents, modes, and lifecycle. On macOS, the child stores ordinary login material in Keychain rather than the relocated credential path; see [accounts](./accounts.md#platform-boundary).

Credentials are state, not data or cache: they are durable, machine-specific, and unsafe to lose silently. Generated settings are state because removing them during a run changes child behavior. Locks are runtime because persistence across reboot makes them stale.

## Group identifiers

| Property        | Rule                                                         |
| --------------- | ------------------------------------------------------------ |
| Character set   | `[a-z0-9_-]` only                                            |
| First character | Lowercase ASCII letter or digit                              |
| Maximum length  | 32 bytes                                                     |
| Derivation      | See [session isolation](../explanation/session-isolation.md) |

Invalid derived identifiers are rejected, never truncated or rewritten. Account identifiers use the same rules.

## Filesystem security

Checks run on every invocation.

| Check                 | Applied to                                                             | On failure               |
| --------------------- | ---------------------------------------------------------------------- | ------------------------ |
| Not a symbolic link   | Every wrapper-managed path component                                   | Refuse with `Permission` |
| Owned by current user | Every wrapper-managed path component                                   | Refuse with `Permission` |
| Expected file type    | Every wrapper-managed path                                             | Refuse with `Permission` |
| Mode `0700`           | Wrapper-managed directories                                            | Correct, then proceed    |
| Mode `0600`           | Wrapper-owned secret, metadata, settings, provenance, and marker files | Correct, then proceed    |

Metadata checks do not follow symbolic links and validate each component. Managed-directory creation is idempotent.

The wrapper validates the child-owned `.credentials.json` path before relying on its presence, but never changes its mode, rewrites it, or follows it to read credential content.

## Atomic writes

Wrapper-owned files whose partial content would be misread use a temporary file in the same directory, flush, mode-setting, then atomic rename. This covers:

- `auth-mode.json` and `oauth-token`;
- generated settings and composition provenance;
- session metadata;
- the last-used marker.

The child-owned `.credentials.json` is explicitly excluded.

## Cleanup

Stale-group pruning is conservative:

- only directories below `groups/` are candidates;
- symbolic links are never followed;
- a possibly active group is retained;
- pruning is opt-in and reported;
- account-wide `config/`, mode metadata, and token storage are never pruned.

`account remove` removes the local account tree, including child-owned config and all groups. It stops local use but does not claim to revoke a token upstream.

## Diagnostics

`doctor` reports resolved base directories, environment-versus-default provenance, selected account and group paths, and wrapper-managed security checks. Child credential content is never inspected or emitted.
