# Logging and output

Every byte the wrapper writes, and where it goes.

This describes normative design. The crate is pre-implementation.

## The stream contract

**Standard output carries the result and nothing else.** Standard error carries everything else.

The test is whether a user could pipe the command into another program. If a byte would corrupt that pipe, it does not belong on standard output.

| Class                     | Stream                                  | Notes                                                                                                                                                                              |
| ------------------------- | --------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A wrapper verb's result   | stdout                                  | Text, or JSON when the verb was given `--json`                                                                                                                                     |
| Progress, status, prompts | stderr                                  | Never stdout, even when interactive. Which verbs prompt at all, and what happens with no terminal, is in [the CLI surface](./cli-surface.md#confirmation-and-non-interactive-use). |
| Warnings                  | stderr                                  |                                                                                                                                                                                    |
| Errors                    | stderr                                  | Four-part shape; see [exit codes](./exit-codes.md)                                                                                                                                 |
| Log records               | Log file, optionally mirrored to stderr | See below                                                                                                                                                                          |
| The child's output        | Inherited                               | The wrapper never intercepts it                                                                                                                                                    |

**During a passthrough invocation the wrapper writes nothing to standard output.** Not a banner, not a progress line, not a "launching claude" notice. The child's standard output is the user's data stream and the wrapper is not entitled to a byte of it. Wrapper diagnostics during a passthrough go to standard error, where they are already interleaved with the child's.

Every terminal write goes through **one output writer**, owned by the context. Direct print macros are forbidden outside that writer and the entry point, and a lint enforces it; see [testing and quality](./testing-and-quality.md). One writer is what makes JSON mode, `--quiet`, and colour handling work uniformly instead of being reimplemented per command.

## Machine output

`--json` makes a wrapper verb emit a single JSON document on standard output. It is a mode, not a decoration: in JSON mode, no human-oriented text appears on standard output at all.

The flag is **verb-level** and every verb that produces data declares its own; there is no global `--format`. The reasoning is in [the CLI surface](./cli-surface.md#machine-output-is-not-on-this-table) and [ADR-0024](../decisions/0024-machine-output-is-a-per-verb-flag.md). One writer still renders every document, so the mode behaves identically across verbs even though the flag is declared per verb.

Errors in JSON mode still go to standard error, and carry the `err.kind` from the exit-code matrix so a script can branch without parsing prose.

## Verbosity

| Invocation | Level              |
| ---------- | ------------------ |
| `--quiet`  | Errors only        |
| Default    | Warnings and above |
| `-v`       | Info and above     |
| `-vv`      | Debug and above    |
| `-vvv`     | Trace              |

`--quiet` and `--verbose` together is a usage error, not a silent precedence rule.

`RUST_LOG` is honoured and, when set, **overrides** the flag-derived level. This is deliberate: the flag is the user's coarse control, and the environment variable is the developer's fine one, which needs per-module filtering the flags cannot express. No `CLAUDE_SESSION_LOG` variable is invented — reusing the ecosystem-standard name means existing knowledge transfers.

Verbosity governs the **stderr mirror**. The log file's own level is independent, so a user who reports a bug has a useful log even though they ran without flags.

## Log records

Exactly one subscriber is installed, from the entry point, before anything else runs. Installing a second is a bug.

The default sink is a file under the state base directory, written non-blocking and rotated. It is a file rather than the terminal because a wrapper's diagnostics interleaved with an interactive child's output are unreadable, and because the log's value is in being there _after_ something went wrong.

The stderr mirror is opt-in, driven by verbosity.

Each record is one line, with a stable field set:

| Field      | Content                                                             |
| ---------- | ------------------------------------------------------------------- |
| `ts`       | ISO 8601 timestamp, UTC, millisecond precision                      |
| `level`    | Lower-case                                                          |
| `target`   | The emitting module path                                            |
| `op`       | A short stable operation name — `resolve_child`, `compose_settings` |
| `msg`      | The message                                                         |
| `status`   | How the operation ended — `ok`, `err`, `skipped`                    |
| `dur_ms`   | Elapsed milliseconds, on records that close an `op`                 |
| `err.kind` | On a failure, the stable kind from [exit codes](./exit-codes.md)    |
| …          | Structured fields, flat key-value                                   |

`status`, `dur_ms`, and `err.kind` appear only where they mean something — a record that opens an operation has no duration yet — but where they appear, they carry these names. `err.kind` matters most: it is already the identifier scripts match on, and a log that spells it differently from the diagnostic forces a reader to learn two vocabularies for one failure.

One line per record, with structured fields rather than interpolated prose, because both a human with `grep` and a program with a parser can then use it.

**Credentials, tokens, and API keys are never logged**, at any level, in any field. Neither are the child's arguments at default verbosity — a passed-through argument can contain a prompt, a path, or a secret. Argument logging is a trace-level, opt-in behaviour.

## Colour

Applied to stderr text and to stdout only in human format. Never in JSON mode. Resolved in this order, first match wins:

1. `NO_COLOR` set to any value — off. Per [the convention](https://no-color.org/).
2. `FORCE_COLOR` set — on, regardless of what the stream is.
3. `TERM` is `dumb` — off.
4. The target stream is not a terminal — off.
5. Otherwise — on.

There is deliberately **no wrapper flag** for colour. `NO_COLOR` is the established convention and costs the child nothing, whereas claiming `--no-color` would take that spelling away from the child for good — a passthrough-contract change needing its own decision record, per [the CLI surface](./cli-surface.md) and [ADR-0003](../decisions/0003-reserve-a-small-wrapper-cli-surface.md).

Colour never carries meaning by itself. Anything colour indicates is also stated in the text, because a redirected stream, a colour-blind reader, and a screen reader all lose it.

## Preflight and `doctor`

`doctor` is the wrapper's self-diagnostic. Its output contract lives here because it _is_ an output surface; the subsystems it inspects are documented in their own pages.

**Every check runs independently, and one failure never aborts the rest.** A `doctor` that stops at the first problem is useless precisely when it is needed, because the first problem is often a consequence of the third.

### One probe set, three call sites

There is exactly **one** catalog of probes, and everything that needs a health answer reads it ([ADR-0018](../decisions/0018-one-probe-set-with-stable-check-ids.md)):

1. **`doctor`** runs the whole catalog and reports.
2. **A command guard** runs the subset that command requires, before doing work.
3. **Any future setup path** runs the subset it needs.

A guard that fails emits its check's remediation **verbatim** — not a paraphrase — so the user reads one wording whether they hit the guard or ran `doctor`. Adding a prerequisite means adding a catalog entry, never bolting a private check onto one call site. Two probe sets drift, and the drift shows up as `doctor` reporting healthy while a command refuses to run.

### The catalog

Each check has a stable kebab-case **id**, a **scope**, a **severity**, and the `err.kind` a failure of it exits with.

| Id                          | Scope   | Severity | `err.kind`           | Passes when                                                     |
| --------------------------- | ------- | -------- | -------------------- | --------------------------------------------------------------- |
| `base-dirs-resolve`         | Host    | Hard     | `Unavailable`        | Config and state resolve to absolute, usable paths              |
| `runtime-dir-present`       | Host    | Soft     | `Unavailable`        | Present; absent is reported, not failed                         |
| `wrapper-config-parses`     | Host    | Hard     | `Config`             | Parses, with no unknown keys                                    |
| `child-binary-resolves`     | Host    | Hard     | `ChildNotFound`      | Found via the ladder in [process runtime](./process-runtime.md) |
| `child-is-executable`       | Host    | Hard     | `ChildNotExecutable` | Executable by the current user                                  |
| `child-version-floor`       | Host    | Soft     | `Unavailable`        | At or above the documented minimum                              |
| `session-root-security`     | Session | Hard     | `Permission`         | Not a symlink, owned by the user, mode `0700`                   |
| `session-identity-derives`  | Session | Soft     | `Unavailable`        | Derives above the process-id fallback rung                      |
| `account-registry-readable` | Session | Soft     | `Io`                 | Readable; accounts have valid seeds                             |
| `credentials-usable`        | Session | Soft     | `Auth`               | The current account has a usable seed or a configured token     |
| `settings-compose`          | Session | Soft     | `DataFormat`         | The active profile resolves and every piece exists              |
| `settings-fresh`            | Session | Soft     | `DataFormat`         | The generated settings are not stale                            |

**Hard** means the wrapper cannot function. **Soft** means a feature is degraded.

Ids are **public API**. Scripts match them and messages cite them, so renaming one is a breaking change and the table grows by appending — the same contract `err.kind` carries in [exit codes](./exit-codes.md). Severity is the only waiver lever: a check that could legitimately be ignored is soft _by definition_, which is why there is no per-invocation ignore flag and a hard check stays an unconditional guarantee.

### Results and exit

A check reports `pass`, `warn`, `fail`, or `skipped`.

A soft check that is inert — a feature the user does not use — reports `skipped` with a reason and **never gates**. Failing `doctor` because the user has not configured accounts they do not want punishes them for not using a feature. Session-scope checks are skipped when no session context applies. **Skips never affect the exit code.**

Exit is `0` when no hard check fails, and otherwise the `err.kind` code of the first failing hard check in catalog order — which is why the table's order is itself contractual.

`doctor --list` prints the catalog — every id, scope, and severity — without running anything, so a script can discover what it may match on.

Output is a human-readable report on standard output plus an overall verdict; `doctor --json` emits every check with its id, scope, severity, status, and message. Each failing check carries the four-part error shape from [exit codes](./exit-codes.md).

The child version floor is a **perishable fact**: the child is externally owned and changes on its own schedule. It is registered in [research tracking](./research-tracking.yaml), and the check is defensive — an unparsable version string is reported, not fatal.

## Further reading

- [`tracing`](https://docs.rs/tracing/) and [`tracing-subscriber`](https://docs.rs/tracing-subscriber/)
- [`no-color.org`](https://no-color.org/)
- [Command Line Interface Guidelines](https://clig.dev/)
