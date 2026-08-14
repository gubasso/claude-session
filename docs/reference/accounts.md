# Accounts

What an account is, how one is selected, and the contract of every `account` subcommand. The design is recorded in [ADR-0025](../decisions/ADR-0025-share-one-native-login-per-account.md), [ADR-0026](../decisions/ADR-0026-store-and-inject-a-long-lived-subscription-token.md), [ADR-0027](../decisions/ADR-0027-ingest-secrets-only-from-stdin-or-a-terminal.md), [ADR-0029](../decisions/ADR-0029-use-a-credential-helper-process-boundary.md), and [ADR-0030](../decisions/ADR-0030-use-account-login-for-wrapper-authentication.md). Paths and permissions live in [XDG storage](./xdg-storage.md).

Both stored modes are implemented, and so are all five subcommands, selection with the last-used marker, local usability reporting, precedence warnings, and credential redaction. The `token_helper` retrieval boundary is the one part of this page that remains normative future design: its argv protocol is unspecified, so the private `oauth-token` file is the only store.

## What an account is

An account is a directory under the state base, named by a user-chosen identifier. There is no global registry: enumerating accounts means reading the accounts directory.

Each account contains `auth-mode.json`. This metadata is part of that account, not a second index, and records:

- `mode`: `login` or `token`;
- `recorded_at`: when the selected authentication was recorded;
- `fingerprint`: `sha256[..8]` of the wrapper-owned token in token mode only.

It also contains `profile.json`, the [bound profile](#the-bound-profile). That is a second file rather than a second field, so rebinding never writes the document whose rename commits a token rotation.

The wrapper owns account selection, mode metadata, and any stored token. The child owns everything below the account's `config/`, including `.credentials.json`. The wrapper never reads, copies, writes, refreshes, synchronizes, or fingerprints a child credential, and caches no child authentication state. `account status` may `stat` the credential path on demand.

It does unlink one, in two places, and nowhere else: [removal](#removal) deletes the account's whole tree, and [a mode switch](#switching-modes) retires the credential it supersedes ([ADR-0101](../decisions/ADR-0101-retire-the-credential-a-mode-switch-supersedes.md)). Both delete a path without reading it, which is why neither is an exception to the sentence above so much as the limit of it.

Account identifiers follow [the identifier rules](./xdg-storage.md#identifiers).

## Selection

The account for an invocation is resolved by the [configuration precedence ladder](./configuration.md#precedence), with one rung appended below it:

1. `--account <name>` — see [the CLI surface](./cli-surface.md#wrapper-owned-flags).
2. [`default_account`](./configuration.md#keys), from the environment then user configuration. A project file cannot supply it ([ADR-0071](../decisions/ADR-0071-restrict-the-project-layer-to-the-profile-key.md)).
3. The last-used marker, written whenever an account-backed run launches.
4. No account resolved. Wrapper verbs that do not launch proceed; a passthrough launch refuses as `Config`.

The marker means “the same account as last time”; any explicit selection overrides it. `account status` reports which rung supplied the answer. It records a selection rather than an outcome, so it is written before the launch, which is also the only place it can be written: the wrapper execs the child and observes nothing afterwards ([ADR-0084](../decisions/ADR-0084-exec-the-child-instead-of-supervising-it.md)).

A passthrough launch requires a selected account before authentication state is inspected or injected ([ADR-0090](../decisions/ADR-0090-require-account-and-profile-before-child-launch.md)).

## The bound profile

An account carries the profile it runs with ([ADR-0096](../decisions/ADR-0096-bind-a-profile-to-an-account.md)). The record is the profile name and when it was recorded, and nothing else: a profile document owns everything about what a profile contains.

`account login` establishes it. The name comes from `--profile <name>`, else from the account's existing binding, else from [`default_profile`](./configuration.md#keys); when none of the three answers, the login refuses as `Config` before the child runs and before any directory is made. The report always names the profile and the layer that supplied it, so an account is never created with that choice left implicit.

`account bind <name> --profile <name>` changes it afterwards, because changing which settings an account uses should not cost a browser flow or a pasted token ([ADR-0097](../decisions/ADR-0097-rebind-a-profile-without-re-authenticating.md)). Both halves are checked before anything is written: an unknown account or a profile with no document is `NoInput`, and the previous binding survives.

The binding takes [one rung](./configuration.md#selecting-the-active-profile) in the profile ladder, under the project file and over user configuration. An account with no binding resolves as it did before and is reported as unbound. A binding whose profile document has gone is reported by `account status` as a warning and by `doctor` as `account-profile-bound`; a launch under it refuses as it does for any unresolvable profile.

Composed settings are unaffected: they are keyed by profile and input digest with no account component ([ADR-0064](../decisions/ADR-0064-key-composed-settings-by-profile-and-input-digest.md)), so two accounts bound to one profile share one entry.

## Stored modes and launch behavior

Mode is chosen by `account login` and resolved deterministically on every later run. Ambient state never changes the stored mode.

| Mode    | Wrapper-provided child environment                                                                                    | Authentication owner                                                    |
| ------- | --------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------- |
| `login` | `CLAUDE_CONFIG_DIR=<this terminal's session directory>`, `CLAUDE_SECURESTORAGE_CONFIG_DIR=<account config directory>` | Child reads and refreshes its saved login, from one file per account    |
| `token` | The same pair, plus `CLAUDE_CODE_OAUTH_TOKEN=<retrieved token>`                                                       | Wrapper stores or retrieves the long-lived token; the child consumes it |

In token mode, `CLAUDE_CODE_OAUTH_TOKEN` outranks a saved login that may also exist in `config/`. The wrapper reports that shadowing. It removes neither credential over it, which is a separate question from [the retirement a mode switch performs](#switching-modes): that one runs because the switch made an artifact unreachable, and shadowing leaves both reachable by whoever points the child at them.

The following ambient child mechanisms outrank the selected subscription account: `ANTHROPIC_API_KEY`, `ANTHROPIC_AUTH_TOKEN`, `apiKeyHelper`, Bedrock, Vertex, and Foundry configuration. They are never wrapper-managed and never stripped from a launch. Which of them the child would prefer over another is the child's own arbitration and is not modelled here. The wrapper observes the environment mechanisms among them, so their presence may produce a warning only:

- before launch, on standard error;
- in `account status`;
- in `doctor`.

A launch is the only thing that rule governs, because there the ambient credential is the user's own choice about their own session. The token verification probe is the exception, and it is not a launch: it asks the child about one specific candidate, so it clears every environment mechanism that would outrank the injected token first. Inheriting one would make the child answer about that credential instead, and any string at all would verify. `apiKeyHelper` is the one it cannot clear, since that lives in the child's settings rather than the environment and the wrapper does not author a settings document to ask a question.

The wrapper never intercepts a slash command.

## Logging in

`account login [name]` is idempotent. It creates the account on the first successful login and safely replaces its mode on later successful runs. A failed first login removes the incomplete account.

There are three ways in and two stored modes. [Native login](#native-login-mode) and [the refresh-token bootstrap](#refresh-token-bootstrap) both end in a child-owned saved login and both record `login`; they differ only in whether a browser or a supplied grant got there. [Token mode](#long-lived-token-mode) is the one that stores a credential of the wrapper's own. `--token` and `--refresh-token` are answers to the same question and are refused together.

### What a login leaves ready

A login commits two things, in this order: the authentication and [the bound profile](#the-bound-profile). The child's own first-run answers are not among them, because the file that holds them does not exist yet: it lives in [this terminal's session directory](./xdg-storage.md#artifact-table), which no login can name and which the first launch creates ([ADR-0105](../decisions/ADR-0105-seed-a-session-at-launch.md)).

### What a launch seeds

A launch that materialises a session directory writes two of the child's own keys into the `.claude.json` inside it.

`hasCompletedOnboarding` is unconditional. The child runs its first-run setup when that key is not `true`, and decides that without consulting authentication — so a brand-new directory sends an authenticated account into a browser sign-in it does not need, and which token mode cannot even absorb, since the stored token shadows whatever that sign-in saves. The wrapper created the directory whose newness makes the child ask, so the wrapper answers ([ADR-0098](../decisions/ADR-0098-seed-the-one-child-key-a-launch-cannot-reach.md)).

The workspace trust keys are written for the launch directory alone, and only while [`auto_trust_cwd`](./configuration.md#keys) is enabled. Marking a directory trusted is a decision about running code found in it, so the key exists to let the user take that answer back; it defaults to enabled, since the alternative is re-approving every project in every terminal.

Nothing else in that file is touched. The write is read-modify-write under the account's credential lock, so the child's own keys and the trust records of every other workspace survive it, and a file that is not a JSON object is refused rather than replaced. That lock excludes another wrapper run rather than a `claude` already running under the account, so the write happens only when this launch would actually change a key — a launch into a directory that already carries both answers reads and returns, leaving a file a running child may be writing alone.

A file the launch cannot read is the one state it cannot repair, since a missing key is simply seeded on the way past. That state is named by `doctor` under `account-launch-ready`, by `account status` as a warning, and on standard error before the exec. The remedy is to move the file aside; the next launch writes a fresh one, and the child rebuilds everything it kept there except its own trust records.

A failure to write does not undo anything already durable. The refusal says what the launch would meet and names the file to deal with.

### Native login mode

The wrapper resolves the account directory and launches the child's own `auth login` with the account `config/` as `CLAUDE_CONFIG_DIR`. It does not implement the browser flow or inspect the result. Native passthrough remains available:

```text
claude-session-rs --account work -- auth login
```

A saved login carries two clocks. The access token expires in hours, and its renewal is a non-event: the child refreshes it without the wrapper or the user taking part. The refresh grant is the clock that ends the login, and only re-authenticating resets it. Neither lifetime is a documented guarantee, and no token prefix identifies which of the two a value belongs to — which is why nothing here infers an expiry from a credential.

Concurrent runs of one account share that saved login. From child version 2.1.211 the child coordinates renewal across the processes holding it, so one refresh happens and the rest observe its result. That coordination is why [ADR-0025](../decisions/ADR-0025-share-one-native-login-per-account.md) shares a login rather than copying it, and why a `login`-mode launch below the floor [fails before the exec](./process-runtime.md#child-version-floor).

An in-TUI `/login` inherits the launch environment and reaches the same child-owned location: the child relocates `.credentials.json` under `CLAUDE_SECURESTORAGE_CONFIG_DIR` when that name is set, and manages that file through `/login` and `/logout`. In token mode it does not silently fail either — from child version 2.1.118, a successful `/login` clears the injected `CLAUDE_CODE_OAUTH_TOKEN` inside the child's own process, so the login it just wrote serves the rest of that session.

That clearing is process-local, which is the whole of the wrapper's concern. It cannot reach the wrapper's stored token, so the next launch injects that token again and shadows the saved login again, and the account's stored mode never changed. A user who ran `/login` and a wrapper that still reports token mode are both correct, and only `account login` changes a mode durably. This is why a token-mode launch says so before the exec rather than after, and why nothing here reads the result of a slash command the wrapper never sees.

### Refresh-token bootstrap

`account login [name] --refresh-token` reaches the same saved login without a browser, for a machine that has none. It exists because [token mode](#long-lived-token-mode) cannot: `claude setup-token` requests `user:inference` and nothing else, and the endpoints behind the child's subscription, organization, email, and usage surfaces all require `user:profile`, which the server refuses that token. No wrapper-side value substitutes for a scope the grant does not hold ([ADR-0100](../decisions/ADR-0100-bootstrap-a-saved-login-from-a-refresh-token.md)).

The wrapper reads one refresh token — from standard input with `--stdin`, otherwise from the controlling terminal with echo disabled, under the same rule as a stored token — and runs the child's own `auth login` with `CLAUDE_CODE_OAUTH_REFRESH_TOKEN` and `CLAUDE_CODE_OAUTH_SCOPES` set, in the account's `config/`. Everything after that is the child's: the exchange, its `.credentials.json`, the account profile it fetches, the organization roles, and its own first-run key. The wrapper reads none of it and commits what a native login commits.

With `--stdin` this needs no terminal, which is the whole point. Without it, the terminal is where the paste comes from, so the terminal is required for the same reason a token paste requires one.

A refresh token is issued only by a completed claude.ai login. On a machine that has one it is `claudeAiOauth.refreshToken` inside the child's `.credentials.json`, under that installation's configuration directory. The wrapper does not mint one, does not read that file to obtain one, and does not speak the endpoint that issues one; the operator supplies it.

`--scopes` declares the scopes the grant was issued with. Without it the wrapper uses the set a claude.ai login is issued with — `org:create_api_key`, `user:profile`, `user:inference`, `user:sessions:claude_code`, `user:mcp_servers`, and `user:file_upload`, read from the authorize URL child 2.1.220 prints — because nearly every refresh token comes from such a login and the child refuses the exchange without a set. This is the one place the wrapper guesses on the user's behalf, and a grant issued differently says so with `--scopes`; asking for a scope the grant does not hold fails the whole exchange, and the child reports that itself. A declared set is validated as RFC 6749 scope tokens and normalized to single spaces, never checked against a vocabulary.

The refresh token is used once and never stored. The exchange may rotate it, so a copy kept beside the account could be dead with nothing local able to tell — which also means repairing such an account later needs a fresh refresh token, or a browser. Nothing under the account holds it afterwards.

What commits is the credential the child left, judged the way a native login judges it: the file is there, and the path to it is safe. A child that exits successfully and leaves none has logged nothing in, so the login fails as `Auth` and a first login removes the account it created.

That gate is a presence test and not a comparison, which bounds what it can say. Against a fresh account it is exact, because there is nothing there to mistake for a new credential. Against an account that already holds a saved login — a repeat login, or a token account carrying one an in-TUI `/login` left — it cannot tell this run's exchange from the credential that was already present, and a child exiting successfully without exchanging would be reported as a success. Nothing in child 2.1.220 does that, since its exchange either completes or exits non-zero, and a `claude` too old to know these variables falls through to a browser flow that cannot complete against the closed standard input this login leaves it. Neither is ruled out by the gate, and the wrapper does not read a child credential to make the comparison that would ([Q-010](../plan/open-questions.md)).

### Long-lived token mode

`account login [name] --token` selects token mode:

1. Without `--stdin`, run `claude setup-token` with inherited standard streams, then read one line from the controlling terminal with echo disabled.
2. With `--stdin`, read one line from standard input and never prompt.
3. Reject empty or multi-line input. Do not parse a prefix or infer token lifetime.
4. Probe the candidate through the child's documented `auth status --json` command, then write the token and mode metadata by [the atomic sequence](./xdg-storage.md#lock-scopes), which owns their order.

Rejecting a multi-line paste means consuming the rest of it as well. A line left queued on the terminal is read by whatever runs next, which after this process exits is the user's shell, so half a credential would arrive there as a command.

While the prompt is up, the interrupt and suspend keys are not signals. Both would end this process without unwinding, leaving the terminal with echo off — an interrupt kills it and a suspend hands the shell back a terminal that does not echo, and neither runs the restore. Instead the interrupt is read as data and answered as a cancellation, which restores the terminal first and then reports; every other control character is refused as malformed. Line editing is untouched, and a prompt nobody wants to answer is left with Enter or end of input, both of which refuse.

`--minted-at` corrects the time used for age and estimated-expiry reporting when a pasted token was minted earlier. Without it, `recorded_at` is the ingest time. Estimated expiry is that time plus 365 days and is always labeled an estimate.

The estimate exists because the child's warning surfaces are asymmetric: it reports an approaching saved-login expiry, but gives no equivalent notice for an injected token, which simply stops working. An estimate derived from ingest time is the best the wrapper can offer without reading the token, which it does not do.

Token material is never accepted through argv, an environment variable, a wrapper file flag, or scraped child output. The wrapper never calls an OAuth endpoint.

The default store is the private `oauth-token` file. An explicitly selected `token_helper` may retrieve it through an argv process boundary once that protocol is specified; helper and file modes never silently fall back to each other.

### Declared subscription plan

A saved login tells the child which subscription it belongs to; an injected token does not. The child reads the plan from the credential it saved for itself, and an injected token replaces that read rather than feeding it, so the child treats the tier as unknown: it describes the session as an API one and picks the model it defaults to without a plan. The credential is a subscription credential throughout, and only the wrapper that injected it is positioned to say which subscription ([ADR-0099](../decisions/ADR-0099-declare-the-plan-a-token-cannot-carry.md)).

So a token login obtains the plan and records it beside the mode. `--plan <plan>` declares it non-interactively; without that option a login with a terminal asks, offering `max`, `pro`, `team`, and `enterprise` — the spellings the child acts on in 2.1.220 — and accepting any other answer too. A `--stdin` login never asks, because standard input is carrying the credential.

A declaration is one to thirty-two characters of letters, digits, hyphens, and underscores, lowercased. That is a shape rule and not a vocabulary: which plans exist is the child's to decide, and a spelling this page does not list is recorded and injected unchanged. The rule exists so a mistyped answer is refused where a person can see it, instead of reaching the child as a value it silently ignores. Lowercasing is about the answer rather than the child, whose own comparison is case-sensitive: someone answering the prompt with `Max` meant `max`, and the wrapper is typing the environment value on their behalf.

At launch, a recorded plan is set in `CLAUDE_CODE_SUBSCRIPTION_TYPE` beside the injected token. Nothing else is set. An ambient value of that name is dropped in token mode whether or not the account declared one, for the reason the injected token displaces an ambient token: it describes some other credential, and letting it stand would answer for an account that declared nothing. Login mode sets neither variable and drops neither.

The plan is a declaration and never a reading. The wrapper does not inspect the token and does not call an endpoint, so nothing verifies that the declared plan is the plan the token actually holds; every surface that shows one words it that way. It rides in the metadata rename that commits a rotation, so a rotation that does not re-declare a plan clears it rather than letting an answer outlive the token it was given about.

Declaring none is allowed. The account works, the launch proceeds, and the condition is named by `doctor` under `account-plan-declared`, by `account status` as a warning, and on standard error before the exec. Every account created before this behaviour existed is in exactly that state.

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

### Switching modes

`account login` against an account that already exists records the mode it was asked for, whichever one was there before. The switch is the whole instruction, so nothing is confirmed and nothing is refused.

That login also retires the credential it supersedes: recording a saved login unlinks the account's `oauth-token`, and recording a token unlinks the child's `.credentials.json`. Only that one file goes; the directory around it is the child's and stays. The unlink follows the metadata rename that commits the mode, so an interruption before that rename leaves the previous credential working rather than the account holding neither ([ADR-0101](../decisions/ADR-0101-retire-the-credential-a-mode-switch-supersedes.md)).

It runs on every login rather than only on a detected switch, which is what repairs an account already carrying a leftover from a switch made before this behaviour existed. A login with nothing to retire is silent about it; one that retires something says so, because the act cannot be undone.

Two logins against one account are ordered by [the credential lock](./xdg-storage.md#lock-scopes), and a saved-login run asks again inside it whether the credential it is about to commit is still there. A run that lost the race refuses without writing a mode and without retiring anything, so the authentication the winner committed survives whole; the diagnostic says which happened and points at `account status` rather than at another login.

Retirement is local. Neither credential is revoked at the provider, and both stay valid until they expire — the same limit [removal](#removal) reports. If the retirement itself fails, the login fails after its credential is already durable: the diagnostic names the path that still holds the previous credential and says the new one is not being undone, because the alternative is destroying a credential that was just proven to work.

## Commands

| Command                 | Arguments                                                                                                     | Reports                                                                                                                                       |
| ----------------------- | ------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| `account login [name]`  | optional account; `--profile`, `--token` or `--refresh-token`, `--stdin`, and the options each selector takes | Selected mode, account path, the bound profile and its provenance, any credential this login retired, and success without credential material |
| `account list`          | —                                                                                                             | Every account's mode and local-state usability, and which one is currently selected                                                           |
| `account status [name]` | named account or selected account                                                                             | Mode metadata, safe token status, child-login presence, selection provenance, and shadowing                                                   |
| `account remove <name>` | account; `--yes`                                                                                              | Whether local state was removed, and that upstream revocation did not occur                                                                   |
| `account bind <name>`   | account; `--profile <name>`                                                                                   | The account, the profile it is now bound to, and when that was recorded                                                                       |

Each declares its own `--json`, as [every verb that produces data does](./logging-and-output.md#machine-output). Data goes to standard output; diagnostics and warnings go to standard error, and a confirmation prompt goes to [the controlling terminal](./cli-surface.md#the-predicate). No subcommand ever prints a credential, at any verbosity or in any format.

## Reports

The [shared document rules](./logging-and-output.md#machine-output) hold for all five: one document per invocation, an absent optional field omitted rather than `null`, and no `schema_version`. Timestamps are RFC 3339 UTC. The human report carries the same facts as sentences rather than as labelled fields, under [presentation rule 7](./presentation.md#the-contract): a fingerprint, a second count, and a probe exit code are in the document above and nowhere else, because a person cannot act on any of the three.

| Subcommand | Always present                                                                           | Present when applicable                                                                                                                                                                          |
| ---------- | ---------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `login`    | `account`, `mode`, `path`, `recorded_at`, `profile`, `profile_source`, `profile_present` | `fingerprint`, `estimated_expiry` — token mode only; `retired_superseded_credential`, `true` on the login that [retired one](#switching-modes) and absent on every other                         |
| `list`     | `accounts[]` of `name`, `mode`, `usable`                                                 | `profile` on a bound entry; `selected` on the one entry; `selection_source` at the top level                                                                                                     |
| `status`   | `account`, `selected`, `mode`, `usable`, `warnings[]`                                    | `profile`, `profile_present`, `profile_source`, `selection_source`, `recorded_at`, `age_seconds`, `estimated_expiry`, `fingerprint`, `metadata_consistent`, `child_login_present`, `child_probe` |
| `remove`   | `account`, `path`, `removed`                                                             | `mode`, `marker_cleared` — when something was removed                                                                                                                                            |
| `bind`     | `account`, `profile`, `recorded_at`                                                      | —                                                                                                                                                                                                |

`account status` reads like this, and every other account report follows the same shape — a heading, one row per fact with its status word, and a closing sentence saying what this run would do:

```text
Account gubasso

  [pass]     This account signs in with a long-lived token this wrapper
             stores, recorded 82 minutes ago. It is estimated to stop
             working around 2027-08-13, which is a guess from when it was
             recorded rather than anything the token itself says.
  [pass]     It is bound to the "work" profile, and that is what this run
             would use.

  This is the account a launch would use, because you named it with
  --account.
```

A row that is not a pass carries the consequence and the command that fixes it, in the same place. The status word pads to a fixed column and prose wraps at column 76, both constants rather than terminal measurements ([presentation](./presentation.md#wrapping-and-columns)).

Four rules the table does not carry:

- `accounts` is present even when empty, because an empty list is the answer rather than an absent field.
- `list` never spawns the child. `usable` is local state alone: the directory passes [the security checks](./xdg-storage.md#filesystem-security), `auth-mode.json` parses, and the mode's stored artifact is present. One probe per account would be one child per account, and `status` is the verb that was asked about a credential.
- `status` is the only subcommand carrying `warnings` as data, because shadowing is its subject. Everywhere else a warning is prose on standard error. Where `metadata_consistent` is `false`, `age_seconds` and `estimated_expiry` are omitted rather than computed from a mint time that is not the token's.
- No field reports upstream revocation. It would be `false` in both modes forever, discriminating nothing ([ADR-0051](../decisions/ADR-0051-let-every-surface-element-discriminate.md)); the fact is a sentence on standard error.
- A discovered directory whose `auth-mode.json` is absent or malformed reports `mode: invalid` and `usable: false` in `list`. `invalid` is a report value only and is never written to metadata.

`selection_source` names the rung of [the ladder](#selection) that answered: `flag`, `environment`, `user-config`, `marker`, or `none`. `child_probe` is `{ status, exit_code }` where `status` is `ok`, `failed`, or `unavailable`, and `exit_code` is present only when a child ran.

## Removal

`remove` deletes the account directory and everything beneath it, and nothing else; [XDG storage](./xdg-storage.md#cleanup-and-recovery) owns the inventory of what goes and what stays. Removal confirms unless `--yes` is present.

Before prompting, standard error warns when the account is selected. The [confirmation contract](./cli-surface.md#the-exchange) owns the exchange and its exit status; declining reports that nothing was removed, as `removed: false` under `--json`. Removing the selected account clears the last-used marker.

Removal takes [the account's credential lock](./xdg-storage.md#lock-scopes), so it cannot interleave with a concurrent `account login`; past the acquisition deadline it exits `LockBusy` having changed nothing. It does not look for a running child and cannot stop one. A session already using the account keeps working, because an unlinked file stays valid through the descriptors already holding it, and standard error says so:

> A session already running on this account keeps working until it exits. Its next start will fail.

Local deletion is not upstream revocation, in either mode: a stored token keeps working wherever else it is used, and a child-owned saved login is not ended by deleting it. Standard error says that unconditionally whenever something was removed, names which of the two is still live, and directs the user to the provider — without naming a page or a URL, since the wrapper cannot verify one. To have the child end its own session first, `claude-session-rs --account <name> -- auth logout` is plain passthrough and costs the wrapper no surface.

## Failure modes

The [exit-code matrix](./exit-codes.md) owns mappings.

| Condition                                                   | `err.kind`                            | Subcommands                   |
| ----------------------------------------------------------- | ------------------------------------- | ----------------------------- |
| Invalid account name or option combination                  | `Usage`                               | all applicable                |
| No subcommand, or an unrecognized one                       | `Usage`                               | bare `account`                |
| No name and no selected account                             | `Usage`                               | `login`                       |
| No selected account for a passthrough launch                | `Config`                              | launch                        |
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
