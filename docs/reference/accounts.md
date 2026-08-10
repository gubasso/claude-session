# Accounts

What an account is, how one is selected, and the contract of every `account` subcommand. The design is recorded in [ADR-0025](../decisions/ADR-0025-share-one-native-login-per-account.md), [ADR-0026](../decisions/ADR-0026-store-and-inject-a-long-lived-subscription-token.md), [ADR-0027](../decisions/ADR-0027-ingest-secrets-only-from-stdin-or-a-terminal.md), [ADR-0029](../decisions/ADR-0029-use-a-credential-helper-process-boundary.md), and [ADR-0030](../decisions/ADR-0030-use-account-login-for-wrapper-authentication.md). Paths and permissions live in [XDG storage](./xdg-storage.md).

This describes normative design. The crate is pre-implementation.

## What an account is

An account is a directory under the state base, named by a user-chosen identifier. There is no global registry: enumerating accounts means reading the accounts directory.

Each account contains `auth-mode.json`. This metadata is part of that account, not a second index, and records:

- `mode`: `login` or `token`;
- `recorded_at`: when the selected authentication was recorded;
- `fingerprint`: `sha256[..8]` of the wrapper-owned token in token mode only.

The wrapper owns account selection, mode metadata, and any stored token. The child owns everything below the account's `config/`, including `.credentials.json`. The wrapper never reads, copies, writes, refreshes, synchronizes, or fingerprints a child credential, and caches no child authentication state. `account status` may `stat` the credential path on demand.

Account identifiers follow [the identifier rules](./xdg-storage.md#identifiers).

## Selection

The account for an invocation is resolved by the [configuration precedence ladder](./configuration.md#precedence), with one rung appended below it:

1. `--account <name>` — see [the CLI surface](./cli-surface.md#wrapper-owned-flags).
2. [`default_account`](./configuration.md#keys), from the environment then user configuration. A project file cannot supply it ([ADR-0071](../decisions/ADR-0071-restrict-the-project-layer-to-the-profile-key.md)).
3. The last-used marker, written whenever an account-backed run launches.
4. Nothing. Verbs that need an account fail; verbs that do not proceed.

The marker means “the same account as last time”; any explicit selection overrides it. `account status` reports which rung supplied the answer. It records a selection rather than an outcome, so it is written before the launch, which is also the only place it can be written: the wrapper execs the child and observes nothing afterwards ([ADR-0084](../decisions/ADR-0084-exec-the-child-instead-of-supervising-it.md)).

A passthrough with no selected account receives neither wrapper authentication variable.

## Stored modes and launch behavior

Mode is chosen by `account login` and resolved deterministically on every later run. Ambient state never changes the stored mode.

| Mode    | Wrapper-provided child environment                                             | Authentication owner                                                    |
| ------- | ------------------------------------------------------------------------------ | ----------------------------------------------------------------------- |
| `login` | `CLAUDE_CONFIG_DIR=<account config directory>`                                 | Child reads and refreshes its saved login                               |
| `token` | The same `CLAUDE_CONFIG_DIR`, plus `CLAUDE_CODE_OAUTH_TOKEN=<retrieved token>` | Wrapper stores or retrieves the long-lived token; the child consumes it |

In token mode, `CLAUDE_CODE_OAUTH_TOKEN` outranks a saved login that may also exist in `config/`. The wrapper reports that shadowing but does not remove either credential.

The following ambient child mechanisms outrank the selected subscription account: `ANTHROPIC_API_KEY`, `ANTHROPIC_AUTH_TOKEN`, `apiKeyHelper`, Bedrock, Vertex, and Foundry configuration. They are never wrapper-managed and never stripped. Their presence may produce a warning only:

- before launch, on standard error;
- in `account status`;
- in `doctor`.

The wrapper never intercepts a slash command.

## Logging in

`account login [name]` is idempotent. It creates the account on the first successful login and safely replaces its mode on later successful runs. A failed first login removes the incomplete account.

### Native login mode

The wrapper resolves the account directory and launches the child's own `auth login` with the account `config/` as `CLAUDE_CONFIG_DIR`. It does not implement the browser flow or inspect the result. Native passthrough remains available:

```text
claude-session --account work -- auth login
```

A saved login carries two clocks. The access token expires in hours, and its renewal is a non-event: the child refreshes it without the wrapper or the user taking part. The refresh grant is the clock that ends the login, and only re-authenticating resets it. Neither lifetime is a documented guarantee, and no token prefix identifies which of the two a value belongs to — which is why nothing here infers an expiry from a credential.

Concurrent runs of one account share that saved login. From child version 2.1.211 the child coordinates renewal across the processes holding it, so one refresh happens and the rest observe its result. That coordination is why [ADR-0025](../decisions/ADR-0025-share-one-native-login-per-account.md) shares a login rather than copying it, and why a `login`-mode launch below the floor [fails before the exec](./process-runtime.md#child-version-floor).

An in-TUI `/login` inherits the launch environment and is expected to address the same child-owned location, but that exact child behavior is externally unverified and tracked in [research tracking](./research-tracking.yaml). What `/login` does while token mode is injecting `CLAUDE_CODE_OAUTH_TOKEN` is unverified for a second reason: the injected token outranks any login it writes, so an apparent success there may change nothing the child goes on to use.

### Long-lived token mode

`account login [name] --token` selects token mode:

1. Without `--stdin`, run `claude setup-token` with inherited standard streams, then read one line from the controlling terminal with echo disabled.
2. With `--stdin`, read one line from standard input and never prompt.
3. Reject empty or multi-line input. Do not parse a prefix or infer token lifetime.
4. Probe the candidate through the child's documented `auth status --json` command, then write the token and mode metadata by [the atomic sequence](./xdg-storage.md#lock-scopes), which owns their order.

`--minted-at` corrects the time used for age and estimated-expiry reporting when a pasted token was minted earlier. Without it, `recorded_at` is the ingest time. Estimated expiry is that time plus 365 days and is always labeled an estimate.

The estimate exists because the child's warning surfaces are asymmetric: it reports an approaching saved-login expiry, but gives no equivalent notice for an injected token, which simply stops working. An estimate derived from ingest time is the best the wrapper can offer without reading the token, which it does not do.

Token material is never accepted through argv, an environment variable, a wrapper file flag, or scraped child output. The wrapper never calls an OAuth endpoint.

The default store is the private `oauth-token` file. An explicitly selected `token_helper` may retrieve it through an argv process boundary once that protocol is specified; helper and file modes never silently fall back to each other.

### Token lifecycle

`account status` may report:

- mode and `recorded_at`;
- age and estimated expiry;
- token fingerprint `sha256[..8]`;
- results from the child's documented status probe;
- selected-account provenance and shadowing warnings.

It never reports the token, token prefix, or any child-credential content or fingerprint.

Rotation verifies the candidate before it writes anything, and [the metadata rename commits it](./xdg-storage.md#lock-scopes). A failure before that rename leaves a usable account; a crash between the two renames leaves the new token described by stale metadata, so the recorded fingerprint no longer matches. `status` reports that mismatch and withholds age and estimated expiry rather than computing them from a mint time that is not the token's; launch proceeds, since the token itself was proven to work. The next `account login` repairs the pair.

`account remove` deletes local use but cannot revoke a token upstream; [its report says so](#removal).

## Commands

| Command                 | Arguments                                                                          | Reports                                                                                     |
| ----------------------- | ---------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| `account login [name]`  | optional account; `--token`, `--stdin`, and token-time correction where applicable | Selected mode, account path, and success without credential material                        |
| `account list`          | —                                                                                  | Every account's mode and local-state usability, and which one is currently selected         |
| `account status [name]` | named account or selected account                                                  | Mode metadata, safe token status, child-login presence, selection provenance, and shadowing |
| `account remove <name>` | account; `--yes`                                                                   | Whether local state was removed, and that upstream revocation did not occur                 |

Each declares its own `--json`, as [every verb that produces data does](./logging-and-output.md#machine-output). Data goes to standard output; diagnostics and warnings go to standard error, and a confirmation prompt goes to [the controlling terminal](./cli-surface.md#the-predicate). No subcommand ever prints a credential, at any verbosity or in any format.

## Reports

The [shared document rules](./logging-and-output.md#machine-output) hold for all four: one document per invocation, an absent optional field omitted rather than `null`, and no `schema_version`. Timestamps are RFC 3339 UTC. The human report carries the same fields as labelled lines.

| Subcommand | Always present                                        | Present when applicable                                                                                                                          |
| ---------- | ----------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| `login`    | `account`, `mode`, `path`, `recorded_at`              | `fingerprint`, `estimated_expiry` — token mode only                                                                                              |
| `list`     | `accounts[]` of `name`, `mode`, `usable`              | `selected` on the one entry; `selection_source` at the top level                                                                                 |
| `status`   | `account`, `selected`, `mode`, `usable`, `warnings[]` | `selection_source`, `recorded_at`, `age_seconds`, `estimated_expiry`, `fingerprint`, `metadata_consistent`, `child_login_present`, `child_probe` |
| `remove`   | `account`, `path`, `removed`                          | `mode`, `marker_cleared` — when something was removed                                                                                            |

Four rules the table does not carry:

- `accounts` is present even when empty, because an empty list is the answer rather than an absent field.
- `list` never spawns the child. `usable` is local state alone: the directory passes [the security checks](./xdg-storage.md#filesystem-security), `auth-mode.json` parses, and the mode's stored artifact is present. One probe per account would be one child per account, and `status` is the verb that was asked about a credential.
- `status` is the only subcommand carrying `warnings` as data, because shadowing is its subject. Everywhere else a warning is prose on standard error. Where `metadata_consistent` is `false`, `age_seconds` and `estimated_expiry` are omitted rather than computed from a mint time that is not the token's.
- No field reports upstream revocation. It would be `false` in both modes forever, discriminating nothing ([ADR-0051](../decisions/ADR-0051-let-every-surface-element-discriminate.md)); the fact is a sentence on standard error.

`selection_source` names the rung of [the ladder](#selection) that answered: `flag`, `environment`, `project-config`, `user-config`, `marker`, or `none`. `child_probe` is `{ status, exit_code }` where `status` is `ok`, `failed`, or `unavailable`, and `exit_code` is present only when a child ran.

## Removal

`remove` deletes the account directory and everything beneath it, and nothing else; [XDG storage](./xdg-storage.md#cleanup-and-recovery) owns the inventory of what goes and what stays. Removal confirms unless `--yes` is present.

Before prompting, standard error warns when the account is selected. The [confirmation contract](./cli-surface.md#the-exchange) owns the exchange and its exit status; declining reports that nothing was removed, as `removed: false` under `--json`. Removing the selected account clears the last-used marker.

Removal takes [the account's credential lock](./xdg-storage.md#lock-scopes), so it cannot interleave with a concurrent `account login`; past the acquisition deadline it exits `LockBusy` having changed nothing. It does not look for a running child and cannot stop one. A session already using the account keeps working, because an unlinked file stays valid through the descriptors already holding it, and standard error says so:

> A session already running on this account keeps working until it exits. Its next start will fail.

Local deletion is not upstream revocation, in either mode: a stored token keeps working wherever else it is used, and a child-owned saved login is not ended by deleting it. Standard error says that unconditionally whenever something was removed, names which of the two is still live, and directs the user to the provider — without naming a page or a URL, since the wrapper cannot verify one. To have the child end its own session first, `claude-session --account <name> -- auth logout` is plain passthrough and costs the wrapper no surface.

## Failure modes

The [exit-code matrix](./exit-codes.md) owns mappings.

| Condition                                                   | `err.kind`                            | Subcommands                   |
| ----------------------------------------------------------- | ------------------------------------- | ----------------------------- |
| Invalid account name or option combination                  | `Usage`                               | all applicable                |
| No subcommand, or an unrecognized one                       | `Usage`                               | bare `account`                |
| No name and no selected account                             | `Usage`                               | `login`                       |
| Named account does not exist                                | `NoInput`                             | `status`, `remove`            |
| Required terminal is unavailable                            | `Unavailable`                         | interactive `login`, `remove` |
| Child login, token validation, or liveness probe fails      | `Auth`                                | `login`                       |
| Mode metadata, selected storage, or child login is unusable | `Auth`                                | launch                        |
| Symlink, owner, or mode validation fails                    | `Permission`                          | all                           |
| Account-tree or helper I/O fails                            | `Io`                                  | all                           |
| Child resolution or execution fails                         | `ChildNotFound`, `ChildNotExecutable` | `login`                       |
| The credential lock is still held at the deadline           | `LockBusy`                            | `remove`                      |

`list` and `status` are [inspection verbs](./exit-codes.md#exit-regimes-by-verb): they exit `0` whatever they find, including no accounts at all, nothing selected, and unusable authentication. A child answer is data in their reports and never their exit; an invocation that tries to use that authentication is what fails.

`login` is the one subcommand that spawns the child, and it keeps its own code from the matrix rather than the child's — it is the child's caller, not its passthrough ([ADR-0068](../decisions/ADR-0068-spawn-the-child-as-a-subroutine.md)). Where the child produced the failure, the diagnostic names that command and its status, and the JSON error document carries `child_exit`.

## Diagnostics

The stable `account-registry-readable` and `credentials-usable` checks cover account directories, mode metadata, selected storage, and mode-aware usability. Both are soft and skipped when no account exists. See [the probe catalog](./doctor.md#the-catalog).
