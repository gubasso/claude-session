# Preflight and `doctor`

The probe catalog, what each check reads, the remediation it prints, and how a run collapses into one exit code.

This describes normative design. The crate is pre-implementation.

The catalog below has three consumers and only one of them is an output surface, which is why it lives here rather than in [logging and output](./logging-and-output.md): a reader holding a check id is asking a health question, not a formatting one. That page still owns the streams and the document rules, and [presentation](./presentation.md) owns the appearance rules this one defers to.

```text
claude-session doctor [--json] [--list] [--strict]
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

Each check has a stable kebab-case id, a scope, a severity, and the `err.kind` a failure of it exits with.

| Id                          | Scope   | Severity | `err.kind`           | Passes when                                                                         |
| --------------------------- | ------- | -------- | -------------------- | ----------------------------------------------------------------------------------- |
| `base-dirs-resolve`         | Host    | Hard     | `Unavailable`        | Config and state resolve to absolute, usable paths                                  |
| `runtime-dir-present`       | Host    | Soft     | `Unavailable`        | Present; absent is reported, not failed                                             |
| `wrapper-config-parses`     | Host    | Hard     | `Config`             | Parses, with no unknown keys                                                        |
| `child-binary-resolves`     | Host    | Hard     | `ChildNotFound`      | Found via the ladder in [process runtime](./process-runtime.md)                     |
| `child-is-executable`       | Host    | Hard     | `ChildNotExecutable` | Executable by the current user                                                      |
| `child-version-floor`       | Host    | Soft     | `Unavailable`        | At or above the documented minimum                                                  |
| `storage-paths-no-symlinks` | Session | Hard     | `Permission`         | No existing wrapper-managed path component is a symbolic link                       |
| `storage-paths-owned`       | Session | Hard     | `Permission`         | Every existing wrapper-managed path component is owned by the current user          |
| `storage-paths-typed`       | Session | Hard     | `Permission`         | Every existing wrapper-managed path has the file type the artifact table assigns it |
| `storage-directory-modes`   | Session | Hard     | `Permission`         | Every wrapper-managed directory has mode `0700`, after automatic correction         |
| `storage-secret-modes`      | Session | Hard     | `Permission`         | Every wrapper-owned file assigned mode `0600` has that mode, after correction       |
| `account-registry-readable` | Session | Soft     | `Io`                 | Account directories and auth-mode metadata are readable and parse                   |
| `credentials-usable`        | Session | Soft     | `Auth`               | The current account's selected login or token mode is usable                        |
| `settings-compose`          | Session | Hard     | `NoInput`            | A resolved profile, and every piece it names, exists                                |
| `settings-entry-consistent` | Session | Hard     | `DataFormat`         | A materialized entry's recorded digest matches the one its inputs recompute         |

Hard means the wrapper cannot function. Soft means a feature is degraded.

The five `storage-*` checks are five ids rather than one because each [storage condition](./xdg-storage.md#filesystem-security) has a different remedy, and a check owns exactly one remediation. Collapsing them would leave one id owning three unrelated instructions, which is the drift the verbatim rule exists to prevent.

Ids are public API. Scripts match them and messages cite them, so renaming one is a breaking change and the table grows by appending — the same contract `err.kind` carries in [exit codes](./exit-codes.md). Severity is the only waiver lever: a check that could legitimately be ignored is soft by definition, which is why there is no per-invocation ignore flag and a hard check stays an unconditional guarantee.

## Remediations

Each failing check owns one remediation template. It is the Hint of the [error shape](./exit-codes.md#error-message-shape); What, Where, and Why are computed from the failure. `{path}`, `{expected_type}`, `{actual_type}`, `{expected_mode}`, `{account}`, `{key}`, `{profile}`, `{version}`, and `{minimum}` are substituted without changing the surrounding wording — "verbatim" means the same template and the same substitution rules at both call sites, not that a runtime path cannot be inserted.

Where one command fixes the condition, the remedy is that command, on its own line, in the form `git` uses for dubious ownership. Where no single command is safe, it is not invented: a bad remedy is worse than a precise description, which is why `storage-paths-owned` below does not print a `chown`.

Every check that can fail has a row here. A guard cannot quote a remediation that lives somewhere else, and a template scattered into the subsystem pages would be the second home the verbatim rule exists to prevent — so the catalog and its wordings sit in one table.

| Check                       | Remediation                                                                                                                                                                                                                               |
| --------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `base-dirs-resolve`         | `XDG_CONFIG_HOME` and `XDG_STATE_HOME` must be absolute paths, or unset so the defaults apply. Run `claude-session doctor` to see what each resolved to.                                                                                  |
| `wrapper-config-parses`     | `{key}` in `{path}` is not a configuration key. Remove it, or correct it to one of the keys [configuration](./configuration.md#keys) lists.                                                                                               |
| `child-binary-resolves`     | No `claude` was found. Install it, put it on `PATH`, or set `child_bin` to its absolute path — [process runtime](./process-runtime.md#child-resolution) gives the order the two rungs are tried in.                                       |
| `child-is-executable`       | `{path}` exists but the current user cannot execute it. Grant execute permission, or point `child_bin` at a different binary.                                                                                                             |
| `child-version-floor`       | The resolved `claude` reports `{version}`, below the `{minimum}` this wrapper is designed against. Upgrade it; until then, saved-login mode is refused and token mode still works.                                                        |
| `storage-paths-no-symlinks` | Move the symbolic link at `{path}` aside and recreate the expected `{expected_type}` there, restoring only content you trust.                                                                                                             |
| `storage-paths-owned`       | `{path}` is owned by another user, which usually means a restored backup or a file created under `sudo`. Do not change its owner in place — move it aside and let the wrapper recreate it as you.                                         |
| `storage-paths-typed`       | `{path}` is a `{actual_type}` and this location must be a `{expected_type}`. Move it aside and let the wrapper recreate it; nothing under this path is unrecoverable except an account login.                                             |
| `storage-directory-modes`   | Could not restrict `{path}` to mode `{expected_mode}`. Check that it is on a filesystem supporting Unix permissions and was created by the current user.                                                                                  |
| `storage-secret-modes`      | Could not restrict `{path}` to mode `{expected_mode}`. Move the file to storage that supports Unix permissions before using it again.                                                                                                     |
| `account-registry-readable` | The account registry under `{path}` could not be read. Check that it exists and is readable; if it is missing entirely, `claude-session account login {account}` recreates it.                                                            |
| `credentials-usable`        | The stored credential for `{account}` is missing, expired, or refused. Run `claude-session account login {account}` to replace it.                                                                                                        |
| `settings-compose`          | `{path}`, named by profile `{profile}`, does not exist. Create it, correct the name in the profile, or select a different profile.                                                                                                        |
| `settings-entry-consistent` | The composed entry at `{path}` does not match the digest its inputs recompute, so it was neither opened nor overwritten. Move it aside; the next launch composes a fresh one. Report this — an entry is written once and never rewritten. |

On a credential path — `oauth-token`, `auth-mode.json`, or the child's `.credentials.json` — the `storage-paths-no-symlinks` template appends one clause, because a link there means something else may have read the secret:

> Anything holding that link may have read this account's credential. Treat it as exposed: run `claude-session account login {account}` for a fresh one, and revoke the old one at the provider.

It appends nowhere else. Claiming exposure over a link on an ordinary metadata path would be crying wolf.

One check carries no template: `runtime-dir-present` reports absence without failing on it, so there is nothing for a Hint to answer. The two mode checks do carry one, because their template covers the case where the correction below could not be applied — not the correction itself.

## Results and exit

A check reports `pass`, `warn`, `fail`, or `skipped`.

A mode check that corrects drift reports `pass`, recording the old and new modes in its detail. It reports `fail` only when the correction cannot be applied, or validation still fails after it. A repair is not an unhealthy state, and `doctor --strict` does not fail on one: the distinction is [`fsck(8)`'s](./prior-art.md#health-checks-and-remediation), which separates errors corrected from errors left uncorrected rather than inventing a second check. This is why mode is a pass condition after correction in the catalog above and "correct, then proceed" in [XDG storage](./xdg-storage.md#filesystem-security).

A soft check that is inert — a feature the user does not use — reports `skipped` with a reason and never gates. Failing `doctor` because the user has not configured accounts they do not want punishes them for not using a feature. Session-scope checks are skipped when no session context applies. Skips never affect the exit code.

Exit is `0` when no hard check fails, and otherwise the `err.kind` code of the first failing hard check in catalog order — which is why the table's order is itself contractual.

`doctor --strict` adds one rule and nothing else: if the run would have exited `0` but any check reported `warn`, it exits `1` instead. It changes no check, no severity, and no output, and it can never make a passing catalog fail. It exists so a CI gate is one flag rather than a JSON parser; the status itself is owned by [exit codes](./exit-codes.md#the-one-code-outside-the-taxonomy) and [ADR-0034](../decisions/ADR-0034-exit-one-when-doctor-strict-promotes-a-warning.md).

`doctor --list` prints the catalog — every id, scope, and severity — without running anything, so a script can discover what it may match on.

## The report

The report is the verb's result, so it goes to standard output; progress and diagnostics go to standard error, which is what makes `doctor --json 2>/dev/null` safe to pipe.

Checks are grouped by scope in catalog order, and each line carries its status as a bracketed word — `[pass]`, `[warn]`, `[fail]`, `[skipped]` — never a glyph or a colour alone, for the reason [presentation](./presentation.md#the-contract) gives. A `warn` or `fail` is followed by an indented `hint:` line carrying the Hint part of the [error shape](./exit-codes.md#error-message-shape); a `skipped` check states its reason instead. The report ends with one summary line giving the counts and the exit the run produced.

`doctor --json` emits that same run as one document:

```json
{
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
  "summary": { "total": 15, "passed": 15, "warned": 0, "failed": 0, "skipped": 0, "hard_failures": 0 },
  "schema_version": 1
}
```

`hint` and `kind` appear on a `warn` or `fail` only, and `reason` only on a `skipped` — so no check ever carries both. `kind` is the `err.kind` from [exit codes](./exit-codes.md). Omitted rather than `null`, per [machine output](./logging-and-output.md#machine-output). `summary.hard_failures` is the field that predicts the exit: zero means `0`.

This is the one document carrying `schema_version`, because its check ids are the public identifiers a script matches on and nothing else the wrapper emits makes that promise.

## The child's own report

`doctor` ends by running `claude doctor` and passing its output through unmodified, after the wrapper's summary line and under the delimiter [logging and output](./logging-and-output.md#composed-output) owns. The child's report is never parsed, reformatted, or summarized: the wrapper claims the verb name only because it composes with the child's rather than replacing it ([ADR-0045](../decisions/ADR-0045-compose-doctor-with-the-child-report.md)), which it is allowed to do because both reports only read and print ([ADR-0079](../decisions/ADR-0079-compose-every-overlapping-surface-with-the-child.md)).

Its exit status enters the catalog as one soft check — zero passes, anything else warns — so it can promote under `--strict` but can never turn a healthy wrapper into a hard failure over a program the wrapper does not own. Under `--json` the child's report is one opaque string field beside its status, which is what keeps the document's schema independent of the child's formatting. If the child cannot be resolved or spawned at all, that is already a hard check of the wrapper's own and this section is skipped with that reason.

The child version floor is a perishable fact: the child is externally owned and changes on its own schedule. It is registered in [research tracking](./research-tracking.yaml), and the check is defensive — an unparsable version string is reported, not fatal.

Mode-aware probes also report ambient-auth shadowing, token-over-login shadowing, and unverified or below-floor child versions as warnings. These use the existing catalog and report model and do not add unstable check ids. A below-floor version is a `doctor` warning but a hard launch failure in `login` mode; see [ADR-0031](../decisions/ADR-0031-enforce-the-child-refresh-lock-version-floor.md) and [process runtime](./process-runtime.md#child-version-floor).
