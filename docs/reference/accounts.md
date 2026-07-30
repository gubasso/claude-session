# Accounts

What an account is, how one is selected, and the contract of every `account` subcommand. For the reasoning behind the seed-and-copy model, see [ADR-0011](../decisions/0011-isolate-credentials-by-seed-and-session.md); for where the files live, [XDG storage](./xdg-storage.md).

This describes normative design. The crate is pre-implementation.

## What an account is

An account is a **directory** under the state base, named by an identifier the user chose, holding a credential seed. There is no registry file, no metadata document, and no derived identity — the directory is the account, and enumerating accounts is reading one directory.

That is deliberate. A registry file is a second source of truth that can disagree with the filesystem, and reconciling the two is a failure mode with no good answer. The paths, modes, and lifetimes are in [XDG storage](./xdg-storage.md); account identifiers follow the group-identifier rules stated there.

The wrapper never parses a credential and never learns who the account belongs to. It knows a seed is present, when the account was last used, and whether the file passes its ownership checks. Anything more would mean reading a secret it has no reason to read.

## Selection

The account for an invocation is resolved by the [configuration precedence ladder](./configuration.md#precedence), with one rung appended below it:

1. `--account <name>` — see [the CLI surface](./cli-surface.md#wrapper-owned-flags).
2. The environment, then project configuration, then user configuration.
3. The **last-used marker**, written whenever a session runs.
4. Nothing. Verbs that need an account fail; verbs that do not, proceed.

The marker is the bottom rung rather than a mode: it means "the same account as last time" for a user who has only one, and it is overridden by any explicit statement. `account status` reports which rung supplied the answer, for the same reason `config view` reports provenance — "why is it using that account?" should be answerable in one command.

## Authenticating

`account add` and `account refresh` both authenticate, and both do it the same way: create the account directory, run **the child's own login** in a scratch directory pointed at by the child's configuration-directory variable, then copy the credential it produced into the account's seed and remove the scratch copy. The wrapper never implements a login flow, never handles the browser, and never sees the exchange. See [ADR-0011](../decisions/0011-isolate-credentials-by-seed-and-session.md).

The seed is written atomically and the scratch directory is removed whether login succeeded or not.

`add` is **transactional**: an account directory created by a login that then failed is removed again. A half-created account that `list` reports and every other verb rejects is worse than no account.

Subscription login needs a browser. Where there is none, the API-token path is the escape, and it is first-class rather than a fallback — no flag substitutes for a browser. [The CLI surface](./cli-surface.md#confirmation-and-non-interactive-use) owns what happens when there is no terminal.

## Refreshing and session copies

Each session gets its **own copy** of the seed, and from the moment it is seeded the child owns that file — this is what lets concurrent sessions refresh tokens without corrupting each other, and it is why a token refreshed inside a session does not propagate back.

The consequence is that re-authenticating an account leaves every live session holding the old credential. `account refresh` therefore **deletes the credential copy in every session directory of that account**, so each session re-seeds from the new seed at its next start. It does not delete the session directories themselves, and it does not touch any other account.

Deleting rather than overwriting is what makes this safe for a session that is running right now: a child holding the file open keeps reading through its open descriptor and is not disturbed mid-flight, and the next start finds the file absent and seeds it fresh.

One gap remains, and it is the one [ADR-0011](../decisions/0011-isolate-credentials-by-seed-and-session.md) already names: a child that writes its credential file in the window after the refresh recreates a copy the next start will not replace. Ending and restarting that session clears it. The wrapper does not kill live children to close it.

## Commands

| Command                  | Arguments                                 | Reports                                                                              |
| ------------------------ | ----------------------------------------- | ------------------------------------------------------------------------------------ |
| `account add <name>`     | the new account's identifier              | The account created, and the directory it was created in                             |
| `account list`           | —                                         | Every account: whether it has a usable seed, when it was last used, which is current |
| `account status [name]`  | an account; the selected one when omitted | One account's seed status, last use, and which rung of the ladder selected it        |
| `account remove <name>`  | the account to remove; `--yes`            | Whether the account was removed, and the directory tree that was deleted             |
| `account refresh [name]` | an account; the selected one when omitted | The account re-authenticated, and how many session copies were invalidated           |

All accept `--json`. All write data to standard output and diagnostics, prompts, and warnings to standard error; see [logging and output](./logging-and-output.md).

**No subcommand ever prints a credential**, at any verbosity or in any format. Seed presence and last use are what answer "is this account usable?", and they answer it without reading the secret.

`add` and `remove` are the only account subcommands that need a person present. Which verbs confirm, why, and what happens without a terminal is in [the CLI surface](./cli-surface.md#confirmation-and-non-interactive-use).

### Removal

`remove` deletes the account directory and everything beneath it, including every session directory belonging to that account. It is the only operation in the program that deletes a session directory that is not stale, which is why it confirms.

Before prompting, it warns when the account is the currently selected one, and when any of its session directories was touched recently enough to still be in use. Both warnings go to standard error, ahead of the prompt, so the person answering has read them first.

**Declining is an answer, not a failure.** A declined removal exits `0` and reports that the account was not removed — including in JSON mode, where a subcommand that emitted no document at all would break the one-document rule in [logging and output](./logging-and-output.md#machine-output).

Removing the currently selected account clears the last-used marker. Leaving it pointing at a directory that no longer exists would make the next invocation fail on a name the user never typed.

## Failure modes

Keyed to [the exit-code matrix](./exit-codes.md), which owns the mapping. No `err.kind` is specific to accounts: the matrix classifies by remedy, and every account failure has a remedy already in it. What distinguishes a missing account from a missing configuration file — both `NoInput` — is the diagnostic's **where** clause, which always names the concrete account or path.

| Condition                                                      | `err.kind`                            | Subcommands                   |
| -------------------------------------------------------------- | ------------------------------------- | ----------------------------- |
| The name breaks the identifier rules                           | `Usage`                               | all that take a name          |
| The name is already an account                                 | `Usage`                               | `add`                         |
| No name given and no account selected by any rung              | `Usage`                               | `status`, `refresh`           |
| The named account does not exist                               | `NoInput`                             | `status`, `remove`, `refresh` |
| No terminal and no escape was given                            | `Unavailable`                         | `add`, `remove`               |
| Login exited non-zero, or produced no credential file          | `Auth`                                | `add`, `refresh`              |
| The seed is unreadable, expired, or refused                    | `Auth`                                | `refresh`                     |
| A symlink, ownership, or mode check on the account tree failed | `Permission`                          | all                           |
| Reading or writing the account tree failed                     | `Io`                                  | all                           |
| The child binary could not be resolved, or is not executable   | `ChildNotFound`, `ChildNotExecutable` | `add`, `refresh`              |

Two cases that are deliberately **not** failures:

- **No accounts at all.** `list` reports none and exits `0`. Never having added an account is a state, not an error.
- **An unusable seed, reported by `status`.** `status` is a report; it exits `0` and says the seed is unusable. Only a subcommand that tries to _use_ the credential fails with `Auth`.

## Diagnostics

Two `doctor` checks cover this subsystem — `account-registry-readable` and `credentials-usable`. Both are soft, and both are skipped rather than failed when the user has no accounts. Their catalog entries are in [logging and output](./logging-and-output.md#the-catalog).
