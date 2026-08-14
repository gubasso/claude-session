# Preflight and `doctor`

The probe catalog, what each check reads, the remediation it prints, and how a run collapses into one exit code.

The `doctor` verb, its report, `--list`, and `--strict` are implemented over 22 checks.

The catalog below has three consumers and only one of them is an output surface, which is why it lives here rather than in [logging and output](./logging-and-output.md): a reader holding a check id is asking a health question, not a formatting one. That page still owns the streams and the document rules, and [presentation](./presentation.md) owns the appearance rules this one defers to.

```text
claude-session-rs doctor [--json] [--list] [--strict]
```

All three flags are verb-level, for the reason [the CLI surface](./cli-surface.md#why---yes-is-not-in-the-flag-table) gives, and they combine: `doctor --list --json` is how a script discovers the catalog.

Every check runs independently, and one failure never aborts the rest. A `doctor` that stops at the first problem is useless precisely when it is needed, because the first problem is often a consequence of the third.

## One probe set, three call sites

There is exactly one catalog of probes, and everything that needs a health answer reads it ([ADR-0018](../decisions/ADR-0018-one-probe-set-with-stable-check-ids.md)):

1. `doctor` runs the whole catalog and reports.
2. A command guard runs the subset that command requires, before doing work.
3. Any future setup path runs the subset it needs.

A guard that fails emits its check's remediation verbatim — not a paraphrase — so the user reads one wording whether they hit the guard or ran `doctor`. Adding a prerequisite means adding a catalog entry, never bolting a private check onto one call site. Two probe sets drift, and the drift shows up as `doctor` reporting healthy while a command refuses to run.

## The catalog

Each check has a stable kebab-case id, a scope, a severity, the `err.kind` a failure of it exits with, and the title a person reads instead of the id ([ADR-0094](../decisions/ADR-0094-give-every-check-a-title-and-a-next-action.md)).

| Id                          | Scope   | Severity | `err.kind`           | Passes when                                                                                                                | Title                                    |
| --------------------------- | ------- | -------- | -------------------- | -------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------- |
| `base-dirs-resolve`         | Host    | Hard     | `Unavailable`        | Config and state resolve to absolute, usable paths                                                                         | Wrapper storage locations                |
| `runtime-dir-present`       | Host    | Soft     | `Unavailable`        | Present; absent is reported, not failed                                                                                    | Runtime directory                        |
| `wrapper-config-parses`     | Host    | Hard     | `Config`             | Parses, with no unknown keys                                                                                               | Wrapper configuration                    |
| `child-binary-resolves`     | Host    | Hard     | `ChildNotFound`      | Found via the ladder in [process runtime](./process-runtime.md)                                                            | The claude program                       |
| `child-is-executable`       | Host    | Hard     | `ChildNotExecutable` | Executable by the current user                                                                                             | Permission to run claude                 |
| `child-version-floor`       | Host    | Soft     | `Unavailable`        | At or above the documented minimum                                                                                         | The claude version                       |
| `storage-paths-no-symlinks` | Session | Hard     | `Permission`         | No existing wrapper-managed path component is a symbolic link                                                              | Symbolic links on session paths          |
| `storage-paths-owned`       | Session | Hard     | `Permission`         | Every existing wrapper-managed path component is owned by the current user                                                 | Ownership of session paths               |
| `storage-paths-typed`       | Session | Hard     | `Permission`         | Every existing wrapper-managed path has the file type the artifact table assigns it                                        | File types of session paths              |
| `storage-directory-modes`   | Session | Hard     | `Permission`         | Every wrapper-managed directory has mode `0700`, after automatic correction                                                | Session directory permissions            |
| `storage-secret-modes`      | Session | Hard     | `Permission`         | Every wrapper-owned file assigned mode `0600` has that mode, after correction                                              | Stored secret permissions                |
| `storage-declared-links`    | Session | Hard     | `Permission`         | Every link the wrapper declared resolves to the target it recorded                                                         | Shared links in the session directory    |
| `settings-compose`          | Session | Hard     | `NoInput`            | A resolved profile, and every piece it names, exists                                                                       | Settings pieces named by the profile     |
| `settings-entry-consistent` | Session | Hard     | `DataFormat`         | A materialized entry's recorded digest matches the one its inputs recompute                                                | The composed settings entry              |
| `account-registry-readable` | Session | Soft     | `Io`                 | The account collection can be enumerated safely                                                                            | The account list                         |
| `credentials-usable`        | Session | Soft     | `Auth`               | The selected account has safe metadata and its stored mode's credential artifact                                           | Sign-in for the selected account         |
| `account-profile-bound`     | Session | Soft     | `Config`             | The selected account is bound to a profile, and that profile has a document                                                | The selected account's profile           |
| `settings-profile-valid`    | Session | Hard     | `DataFormat`         | The resolved profile document, and the array strategy table it declares, are structurally valid and applicable             | The profile document                     |
| `account-launch-ready`      | Session | Soft     | `DataFormat`         | The child configuration this terminal would launch with can be read and understood                                         | The selected account's first run         |
| `account-plan-declared`     | Session | Soft     | `Config`             | The selected token account has declared the subscription plan its token belongs to                                         | The selected account's subscription plan |
| `session-terminal-derives`  | Session | Hard     | `Unavailable`        | A terminal or a session leader, inside the namespace that issued it, named the directory this run's child state belongs in | The terminal this session belongs to     |
| `session-assets-linked`     | Session | Soft     | `Config`             | The asset tree holds at least one of the names claude reads from its configuration directory                               | Your own skills, agents, and rules       |

Hard means the wrapper cannot function. Soft means a feature is degraded.

A title is not an identifier. It heads the row a person reads, it is absent from `--json`, and it may be reworded at any time — which is what keeps the breaking-change rule below attached to the id alone.

The six `storage-*` checks are six ids rather than one because each [storage condition](./xdg-storage.md#filesystem-security) has a different remedy, and a check owns exactly one remediation. Collapsing them would leave one id owning three unrelated instructions, which is the drift the verbatim rule exists to prevent.

Ids are public API. Scripts match them and messages cite them, so renaming or removing one is a breaking change, and no id here has ever been either. The table's order is the order results are produced, not the order they were added, so a new check takes the position its producer emits it in and the ids around it keep their meanings — the same contract `err.kind` carries in [exit codes](./exit-codes.md). Severity is the only waiver lever: a check that could legitimately be ignored is soft by definition, which is why there is no per-invocation ignore flag and a hard check stays an unconditional guarantee.

## Remediations

Each failing check owns one consequence and one remediation template. The consequence is what the condition costs the reader; the remediation is the Hint of the [error shape](./exit-codes.md#error-message-shape). What, Where, and Why are computed from the failure. `{path}`, `{expected_type}`, `{actual_type}`, `{expected_mode}`, `{account}`, `{key}`, `{profile}`, `{version}`, and `{minimum}` are substituted without changing the surrounding wording — "verbatim" means the same template and the same substitution rules at both call sites, not that a runtime path cannot be inserted.

Where one command fixes the condition, the remedy is that command, on its own line, in the form `git` uses for dubious ownership. Where no single command is safe, it is not invented: a bad remedy is worse than a precise description, which is why `storage-paths-owned` below does not print a `chown`.

Every check that can fail has a row here. A guard cannot quote a remediation that lives somewhere else, and a template scattered into the subsystem pages would be the second home the verbatim rule exists to prevent — so the catalog and its wordings sit in one table.

Both columns are written for a person, per rule 7 of [presentation](./presentation.md#the-contract): plain sentences, a runnable command where one command fixes the condition, and no Markdown link or relative document path, because a terminal cannot follow either.

| Check                       | Why it matters                                                                                                                                                  | What to do                                                                                                                                                                                                        |
| --------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `base-dirs-resolve`         | The wrapper cannot find its own configuration or state, so nothing it has stored is reachable.                                                                  | Set `XDG_CONFIG_HOME` and `XDG_STATE_HOME` to absolute paths, or unset them so the defaults apply.                                                                                                                |
| `runtime-dir-present`       | Nothing the wrapper does needs it, so it costs the reader nothing. It is reported because a desktop session normally provides one.                              | Nothing — this is the one check that owns no remediation, for the reason below.                                                                                                                                   |
| `wrapper-config-parses`     | The wrapper stopped rather than guess what an unrecognized setting was meant to do.                                                                             | `{key}` in `{path}` is not a setting this wrapper has. Remove it, or replace it with one of the keys the configuration reference lists.                                                                           |
| `child-binary-resolves`     | There is no `claude` to launch, so every wrapped command would fail the same way.                                                                               | Install `claude` and put it on `PATH`, or set `child_bin` to its absolute path. The wrapper tries `child_bin` first and `PATH` second.                                                                            |
| `child-is-executable`       | The program is there, but this user is not allowed to run it.                                                                                                   | Grant execute permission on `{path}`, or point `child_bin` at a different binary.                                                                                                                                 |
| `child-version-floor`       | Saved-login accounts share one login between processes, and `claude` below `{minimum}` does not lock the token refresh that makes sharing safe.                 | Upgrade `claude` to `{minimum}` or newer before using a saved-login account. Token accounts are unaffected and still work below that version.                                                                     |
| `storage-paths-no-symlinks` | A link on a wrapper-managed path can point anywhere, including somewhere another user can read.                                                                 | Move the symbolic link at `{path}` aside and recreate the expected `{expected_type}` there, restoring only content you trust.                                                                                     |
| `storage-paths-owned`       | Another user owns a path this wrapper writes to, so it cannot promise what ends up in it.                                                                       | `{path}` is owned by another user, which usually means a restored backup or a file created under `sudo`. Do not change its owner in place. Move it aside and let the wrapper recreate it as you.                  |
| `storage-paths-typed`       | The wrapper expected one kind of file and found another, so writing there could destroy something.                                                              | `{path}` is a `{actual_type}` and this location must be a `{expected_type}`. Move it aside and let the wrapper recreate it. Nothing under this path is unrecoverable except an account login.                     |
| `storage-directory-modes`   | A directory other users can read would expose this session's state.                                                                                             | Could not restrict `{path}` to mode `{expected_mode}`. Check that it is on a filesystem supporting Unix permissions and was created by the current user.                                                          |
| `storage-secret-modes`      | A stored credential other users can read is a credential to treat as exposed.                                                                                   | Could not restrict `{path}` to mode `{expected_mode}`. Move the file to storage that supports Unix permissions before using it again.                                                                             |
| `storage-declared-links`    | A link the wrapper shares state through now points somewhere else, so this session would read state that is not its own.                                        | `{path}` is a link to `{actual_type}`, and this wrapper made it point somewhere else. Remove the link and let the wrapper recreate it.                                                                            |
| `settings-compose`          | The profile names a settings piece that is not there, so there is nothing to compose.                                                                           | `{path}`, named by profile `{profile}`, does not exist. Create it, correct the name in the profile, or select a different profile.                                                                                |
| `settings-entry-consistent` | A composed entry changed after it was written, and the wrapper will not hand `claude` a file it cannot vouch for.                                               | The entry at `{path}` was neither opened nor overwritten. Move it aside and the next launch composes a fresh one. Please report this: an entry is written once and never rewritten, so something else changed it. |
| `account-registry-readable` | Accounts cannot be listed, so none of them can be selected.                                                                                                     | Make `{path}` a readable, private directory owned by the current user, then run this again.                                                                                                                       |
| `credentials-usable`        | The selected account cannot sign in, so a launch bound to it would fail at the child.                                                                           | Run `claude-session-rs account login {account}` to recreate this account's stored authentication and its local metadata.                                                                                          |
| `account-profile-bound`     | The selected account names no usable profile, so a launch under it refuses before the child starts.                                                             | Run `claude-session-rs account bind {account} --profile <name>` to name the profile this account runs with.                                                                                                       |
| `settings-profile-valid`    | The profile parsed, but it does not describe a composition the wrapper can carry out.                                                                           | The profile at `{path}`, named `{profile}`, is not usable. Correct its layer list or the array strategy it declares, then run the launch again.                                                                   |
| `account-launch-ready`      | The child configuration this terminal would launch with cannot be read, so the launch refuses rather than write into a file it does not understand.             | Move `{path}` aside. The next launch writes a fresh one, and the child rebuilds everything it kept there except its own trust records.                                                                            |
| `account-plan-declared`     | Claude cannot tell which subscription the stored token belongs to, so it describes the session as an API one and picks the model it defaults to without a plan. | Run `claude-session-rs account login {account} --token --plan <plan>` to declare which subscription this account's token belongs to. The login asks for it when `--plan` is omitted.                              |
| `session-terminal-derives`  | Without a terminal to name, this run cannot be given state of its own, and sharing another terminal's is what the separation exists to prevent.                 | Run this from a terminal. A pipeline or a service without one still works when its own process group leads a session.                                                                                             |
| `session-assets-linked`     | An isolated configuration directory reaches none of the skills, agents, or rules you wrote, so claude starts without them.                                      | Put the skills, agents, and other assets you want in every session under `{path}`. Moving an existing collection there is enough.                                                                                 |

On a credential path — `oauth-token`, `auth-mode.json`, or the child's `.credentials.json` — a symlink means something else may have read the secret, so one clause is appended. Only `credentials-usable` appends it today, over the selected account's three credential paths; the wrapper-managed storage checks refuse a link on those paths without it. Ownership, type, and symlink defects on the child-owned credential are reported by `credentials-usable` under its published `Auth` kind, and the diagnostic names the concrete ownership cause:

> Anything holding that link may have read this account's credential. Treat it as exposed: run `claude-session-rs account login {account}` for a fresh one, and revoke the old one at the provider.

It appends nowhere else. Claiming exposure over a link on an ordinary metadata path would be crying wolf.

One check carries no template: `runtime-dir-present` reports absence without failing on it, so there is nothing for a Hint to answer. It still states a consequence, because "this costs you nothing" is the one thing a reader of a warning with no remedy needs told, and leaving the row silent sends them hunting for a fix that does not exist. The two mode checks do carry one, because their template covers the case where the correction below could not be applied — not the correction itself.

## Results and exit

A check reports `pass`, `warn`, `fail`, or `skipped`.

A mode check that corrects drift reports `pass`, recording the old and new modes in its detail. It reports `fail` only when the correction cannot be applied, or validation still fails after it. A repair is not an unhealthy state, and `doctor --strict` does not fail on one: the distinction is [`fsck(8)`'s](./prior-art.md#health-checks-and-remediation), which separates errors corrected from errors left uncorrected rather than inventing a second check. This is why mode is a pass condition after correction in the catalog above and "correct, then proceed" in [XDG storage](./xdg-storage.md#filesystem-security).

A soft check that is inert — a feature the user does not use — reports `skipped` with a reason and never gates. Failing `doctor` because the user has not configured accounts they do not want punishes them for not using a feature. Session-scope checks are skipped when no session context applies. Skips never affect the exit code.

The wrapper level exits `0` when no hard check fails, and otherwise the `err.kind` code of the first failing hard check in catalog order — which is why the table's order is itself contractual.

That is one of three levels a composed run reports, and [the three levels](#the-three-levels) below owns how they combine into the status the process returns.

`doctor --strict` adds one rule and nothing else: if the run would have exited `0` but any check reported `warn`, it exits `1` instead. It changes no check, no severity, and no output, and it can never make a passing catalog fail. It exists so a CI gate is one flag rather than a JSON parser; the status itself is owned by [exit codes](./exit-codes.md#the-one-code-outside-the-taxonomy) and [ADR-0034](../decisions/ADR-0034-exit-one-when-doctor-strict-promotes-a-warning.md).

### The three levels

A composed run has two answers, so it reports three levels, each stating its own status and its own code ([ADR-0085](../decisions/ADR-0085-carry-the-child-report-level-into-the-verdict.md)):

| Level     | Status from                                | Code                                                         |
| --------- | ------------------------------------------ | ------------------------------------------------------------ |
| `wrapper` | this page's catalog                        | `0`, or the first hard failure's `err.kind` in catalog order |
| `child`   | the child's own `doctor` run               | the child's own exit status, verbatim                        |
| `doctor`  | the worse of the two, with `--strict` last | the status the process returns                               |

The child is the application the wrapper exists to run, so its level crosses unchanged: a child that reports a problem makes the verdict `fail`, without `--strict` and with no way to soften it into a warning. The verdict then exits `Unavailable`, because a subroutine verb answers in the wrapper's own matrix ([ADR-0068](../decisions/ADR-0068-spawn-the-child-as-a-subroutine.md)) — the child's own code is reported as data beside it rather than becoming the process status. A wrapper hard failure outranks the child and keeps its own code, because a wrapper defect usually explains the child's.

Every level publishes the counts that produced it. A caller never has to explain the exit with a value the report omits, which is the failure mode this shape exists to prevent.

`doctor --list` prints the catalog without running anything, so a script can discover what it may match on. Human list output groups by scope and gives one line per check, carrying the title, the id, and whether the check is required or optional — the words `hard` and `soft` name a severity model a reader of the list has no reason to hold. `doctor --list --json` is the surface a script reads, and emits one document with `schema_version: 1` and an ordered `checks` array whose objects contain only `id`, `scope`, and `severity`.

## The report

The report is the verb's result, so it goes to standard output; progress and diagnostics go to standard error, which is what makes `doctor --json 2>/dev/null` safe to pipe.

The human report is written for a person and carries no identifier a person did not ask for, per rule 7 of [presentation](./presentation.md#the-contract). Every field it stops printing is in `--json` below, which is where a script was always meant to read.

Checks are grouped by scope in catalog order, under a heading that says what the scope covers. Each row carries its status as a bracketed word — `[pass]`, `[warn]`, `[fail]`, `[skipped]` — never a glyph or a colour alone. The status word pads to a fixed column and prose wraps at column 76, both constants rather than terminal measurements.

```text
Host — this machine, the wrapper's own files, and the claude program

  [pass]     Wrapper storage locations
             Settings in /home/you/.config/claude-session-rs
             State in /home/you/.local/state/claude-session-rs
  [pass]     The claude program
             /nix/store/…/bin/claude
  [warn]     The claude version
             claude reports 2.0.9. Saved-login accounts share one login
             between processes, and claude below 2.1.211 does not lock the
             token refresh that makes sharing safe.
             What to do: upgrade claude to 2.1.211 or newer before using a
             saved-login account. Token accounts are unaffected and still
             work below that version.
             check: child-version-floor

Session — the account and profile a launch would use

  [skipped]  Not applicable: no account exists yet
             What to do: run claude-session-rs account login <name>
             checks: account-registry-readable, credentials-usable,
             account-profile-bound, account-launch-ready,
             account-plan-declared
             session-terminal-derives
             session-assets-linked

Summary

  22 checks: 14 passed, 1 warning, 7 not applicable.
  Nothing is blocking a launch; one warning is worth reading.
  claude's own checkup reported no problems.
```

A passing row is its status word, its title, and at most the paths or values that check observed. A `warn` or `fail` row states what happened and why it matters, then a `What to do:` block carrying the Hint part of the [error shape](./exit-codes.md#error-message-shape). A `skipped` row reads as not applicable, gives its reason, and says what would make the check apply. Any row that is not a pass ends with the `check:` line naming its id, so the public identifier stays reachable without leading the row.

Consecutive checks that are skipped for one identical reason collapse into a single row, whose `checks:` line names every id it stands for. The collapse is presentation only: `--json` always emits one object per check, in catalog order, so the catalog a script sees never changes shape.

The `Summary` section states the three levels [the three levels](#the-three-levels) defines, in prose rather than fields: the counts the wrapper level produced, whether anything blocks a launch, and what the child's own report said. When the verdict is not a pass, one further sentence names the exit status and which level produced it — the property that every level accounts for the process status is kept, in words. When the child never ran, the summary says so and no delimiter or section follows.

`doctor --json` emits that same run as one document, with one object per level:

```json
{
  "status": "fail",
  "exit": 69,
  "wrapper": {
    "status": "pass",
    "checks": [
      {
        "id": "child-binary-resolves",
        "scope": "host",
        "severity": "hard",
        "status": "pass",
        "message": "…",
        "hint": "…",
        "reason": "…",
        "kind": "ChildNotFound"
      }
    ],
    "summary": { "total": 22, "passed": 22, "warned": 0, "failed": 0, "skipped": 0, "hard_failures": 0, "exit": 0 }
  },
  "child": { "status": "fail", "output": "…", "exit": 1 },
  "schema_version": 1
}
```

`hint` and `kind` appear on a `warn` or `fail` only, and `reason` only on a `skipped` — so no check ever carries both. `kind` is the `err.kind` from [exit codes](./exit-codes.md). Omitted rather than `null`, per [machine output](./logging-and-output.md#machine-output). `wrapper.summary.hard_failures` is the field that predicts that level's exit: zero means `0`. The document's own `exit` is the verdict's, and [the three levels](#the-three-levels) says how the two relate.

This is the one document carrying `schema_version`, because its check ids are the public identifiers a script matches on and nothing else the wrapper emits makes that promise.

## The child's own report

`doctor` ends by running `claude doctor` and passing its output through unmodified, after the wrapper's summary line and under the delimiter [logging and output](./logging-and-output.md#composed-output) owns. The child's report is never parsed, reformatted, or summarized: the wrapper claims the verb name only because it composes with the child's rather than replacing it ([ADR-0045](../decisions/ADR-0045-compose-doctor-with-the-child-report.md)), which it is allowed to do because both reports only read and print ([ADR-0079](../decisions/ADR-0079-compose-every-overlapping-surface-with-the-child.md)).

The child result is a level of its own, not a catalog row: it has no stable check id and is excluded from `--list`, ordered hard-failure selection, and every wrapper summary count. Its `status` is `pass`, `fail`, or `skipped`; `output` is one opaque string, `exit` is present for a normal exit, and `reason` is present when there is no code to report. A nonzero exit or a signal is `fail`, and [the three levels](#the-three-levels) carries it into the verdict unsoftened. If the child cannot be resolved or spawned, the corresponding wrapper-owned catalog row carries the hard failure and the child level is skipped with the same prerequisite reason.

`doctor` is the one composed surface that captures the child's output rather than letting it inherit standard output, because the verdict is only knowable once the child has exited and [composed output](./logging-and-output.md#composed-output) requires the wrapper's own bytes to come first. The bytes are replayed unchanged; nothing follows them.

The child version floor is a perishable fact: the child is externally owned and changes on its own schedule. It is registered in [research tracking](./research-tracking.yaml), and the check is defensive — an unparsable version string is reported, not fatal.

Mode-aware probes also report ambient-auth shadowing, token-over-login shadowing, and unverified or below-floor child versions as warnings. These use the existing catalog and report model and do not add unstable check ids. A below-floor version is a `doctor` warning but a hard launch failure in `login` mode; see [ADR-0031](../decisions/ADR-0031-enforce-the-child-refresh-lock-version-floor.md) and [process runtime](./process-runtime.md#child-version-floor).
