# Accounts

What an account is, how one is selected, and the contract of every `account` subcommand. The design is recorded in [ADR-0025](../decisions/0025-share-one-native-login-per-account.md), [ADR-0026](../decisions/0026-store-and-inject-a-long-lived-subscription-token.md), [ADR-0027](../decisions/0027-ingest-secrets-only-from-stdin-or-a-terminal.md), [ADR-0029](../decisions/0029-use-a-credential-helper-process-boundary.md), and [ADR-0030](../decisions/0030-use-account-login-for-wrapper-authentication.md). Paths and permissions live in [XDG storage](./xdg-storage.md).

This describes normative design. The crate is pre-implementation.

## What an account is

An account is a directory under the state base, named by a user-chosen identifier. There is no global registry: enumerating accounts means reading the accounts directory.

Each account contains `auth-mode.json`. This metadata is part of that account, not a second index, and records:

- `mode`: `login` or `token`;
- `recorded_at`: when the selected authentication was recorded;
- `fingerprint`: `sha256[..8]` of the wrapper-owned token in token mode only.

The wrapper owns account selection, mode metadata, and any stored token. The child owns everything below the account's `config/`, including `.credentials.json`. The wrapper never reads, copies, writes, refreshes, synchronizes, or fingerprints a child credential, and caches no child authentication state. `account status` may `stat` the credential path on demand.

Account identifiers follow the group-identifier rules in [XDG storage](./xdg-storage.md).

## Selection

The account for an invocation is resolved by the [configuration precedence ladder](./configuration.md#precedence), with one rung appended below it:

1. `--account <name>` — see [the CLI surface](./cli-surface.md#wrapper-owned-flags).
2. The environment, then project configuration, then user configuration.
3. The last-used marker, written whenever an account-backed run completes.
4. Nothing. Verbs that need an account fail; verbs that do not proceed.

The marker means “the same account as last time”; any explicit selection overrides it. `account status` reports which rung supplied the answer.

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

Concurrent runs of one account share that saved login. From child version 2.1.211 the child coordinates renewal across the processes holding it, so one refresh happens and the rest observe its result. That coordination is why [ADR-0025](../decisions/0025-share-one-native-login-per-account.md) shares a login rather than copying it, and why a `login`-mode launch below the floor [fails before spawn](./process-runtime.md#child-version-floor).

An in-TUI `/login` inherits the launch environment and is expected to address the same child-owned location, but that exact child behavior is externally unverified and tracked in [research tracking](./research-tracking.yaml). What `/login` does while token mode is injecting `CLAUDE_CODE_OAUTH_TOKEN` is unverified for a second reason: the injected token outranks any login it writes, so an apparent success there may change nothing the child goes on to use.

### Long-lived token mode

`account login [name] --token` selects token mode:

1. Without `--stdin`, run `claude setup-token` with inherited standard streams, then read one line from the controlling terminal with echo disabled.
2. With `--stdin`, read one line from standard input and never prompt.
3. Reject empty or multi-line input. Do not parse a prefix or infer token lifetime.
4. Stage the token in private storage, probe it through the child's documented `auth status --json` command, then atomically replace the old token and mode metadata.

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

Rotation is transactional: stage, verify, replace, then discard staging. Failure leaves the old token and metadata intact. `account remove` deletes local use but cannot revoke a token upstream; its report says so.

## Platform boundary

On Linux and Windows, the child stores its ordinary login under `CLAUDE_CONFIG_DIR`; this wrapper remains Unix-only. On macOS, ordinary login persists in Keychain, but Keychain namespacing by config directory is unverified. Token mode is therefore the supported per-process multi-account design on macOS.

## Commands

| Command                 | Arguments                                                                          | Reports                                                                                     |
| ----------------------- | ---------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| `account login [name]`  | optional account; `--token`, `--stdin`, and token-time correction where applicable | Selected mode, account path, and success without credential material                        |
| `account list`          | —                                                                                  | Every account's mode, local-state usability, last use, and current selection                |
| `account status [name]` | named account or selected account                                                  | Mode metadata, safe token status, child-login presence, selection provenance, and shadowing |
| `account remove <name>` | account; `--yes`                                                                   | Whether local state was removed and, for token mode, that upstream revocation did not occur |

All accept `--json`. Data goes to standard output; diagnostics, prompts, and warnings go to standard error. **No subcommand ever prints a credential**, at any verbosity or in any format. See [logging and output](./logging-and-output.md).

## Removal

`remove` deletes the account directory and everything beneath it. Stale-group pruning never removes account-wide config. Removal confirms unless `--yes` is present.

Before prompting, standard error warns when the account is selected or any group directory may still be active. Declining exits `0` and reports that nothing was removed, including as one JSON document. Removing the selected account clears the last-used marker.

Local token deletion is not upstream revocation. Child-owned login revocation remains a child operation.

## Failure modes

The [exit-code matrix](./exit-codes.md) owns mappings.

| Condition                                                   | `err.kind`                            | Subcommands                   |
| ----------------------------------------------------------- | ------------------------------------- | ----------------------------- |
| Invalid account name or option combination                  | `Usage`                               | all applicable                |
| No name and no selected account                             | `Usage`                               | `login`, `status`             |
| Named account does not exist                                | `NoInput`                             | `status`, `remove`            |
| Required terminal is unavailable                            | `Unavailable`                         | interactive `login`, `remove` |
| Child login, token validation, or liveness probe fails      | `Auth`                                | `login`                       |
| Mode metadata, selected storage, or child login is unusable | `Auth`                                | launch                        |
| Symlink, owner, or mode validation fails                    | `Permission`                          | all                           |
| Account-tree or helper I/O fails                            | `Io`                                  | all                           |
| Child resolution or execution fails                         | `ChildNotFound`, `ChildNotExecutable` | `login`                       |

`list` with no accounts exits `0`. `status` reports unusable authentication and exits `0`; an invocation that tries to use it fails.

## Diagnostics

The stable `account-registry-readable` and `credentials-usable` checks cover account directories, mode metadata, selected storage, and mode-aware usability. Both are soft and skipped when no account exists. See [logging and output](./logging-and-output.md#the-catalog).
