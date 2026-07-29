# Logging and output

Every byte the wrapper writes, and where it goes.

This describes normative design. The crate is pre-implementation.

## The stream contract

**Standard output carries the result and nothing else.** Standard error carries everything else.

The test is whether a user could pipe the command into another program. If a byte would corrupt that pipe, it does not belong on standard output.

| Class                     | Stream                                  | Notes                                              |
| ------------------------- | --------------------------------------- | -------------------------------------------------- |
| A wrapper verb's result   | stdout                                  | Text or JSON per `--format`                        |
| Progress, status, prompts | stderr                                  | Never stdout, even when interactive                |
| Warnings                  | stderr                                  |                                                    |
| Errors                    | stderr                                  | Four-part shape; see [exit codes](./exit-codes.md) |
| Log records               | Log file, optionally mirrored to stderr | See below                                          |
| The child's output        | Inherited                               | The wrapper never intercepts it                    |

**During a passthrough invocation the wrapper writes nothing to standard output.** Not a banner, not a progress line, not a "launching claude" notice. The child's standard output is the user's data stream and the wrapper is not entitled to a byte of it. Wrapper diagnostics during a passthrough go to standard error, where they are already interleaved with the child's.

Every terminal write goes through **one output writer**, owned by the context. Direct print macros are forbidden outside that writer and the entry point, and a lint enforces it; see [testing and quality](./testing-and-quality.md). One writer is what makes `--format json`, `--quiet`, and colour handling work uniformly instead of being reimplemented per command.

## Machine output

`--format json` makes a wrapper verb emit a single JSON document on standard output. It is a mode, not a decoration: in JSON mode, no human-oriented text appears on standard output at all.

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

| Field    | Content                                                             |
| -------- | ------------------------------------------------------------------- |
| `ts`     | ISO 8601 timestamp, UTC, millisecond precision                      |
| `level`  | Lower-case                                                          |
| `target` | The emitting module path                                            |
| `op`     | A short stable operation name — `resolve_child`, `compose_settings` |
| `msg`    | The message                                                         |
| …        | Structured fields, flat key-value                                   |

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

| Check                        | Class | Passes when                                                     |
| ---------------------------- | ----- | --------------------------------------------------------------- |
| Base directories resolve     | Hard  | Config and state resolve to absolute, usable paths              |
| Runtime directory            | Soft  | Present; absent is reported, not failed                         |
| Wrapper configuration        | Hard  | Parses, with no unknown keys                                    |
| Child binary resolves        | Hard  | Found via the ladder in [process runtime](./process-runtime.md) |
| Child is executable          | Hard  | Executable by the current user                                  |
| Child version floor          | Soft  | At or above the documented minimum                              |
| Session root security        | Hard  | Not a symlink, owned by the user, mode `0700`                   |
| Session identity derives     | Soft  | Derives above the process-id fallback rung                      |
| Account registry             | Soft  | Readable; accounts have valid seeds                             |
| Credentials                  | Soft  | The current account has a usable seed or a configured token     |
| Settings composition         | Soft  | The active profile resolves and every piece exists              |
| Generated settings freshness | Soft  | Not stale                                                       |

**Hard** means the wrapper cannot function. **Soft** means a feature is degraded.

A soft check that is inert — a feature the user does not use — reports as not-applicable and **never gates**. Failing `doctor` because the user has not configured accounts they do not want punishes them for not using a feature.

Output is a human-readable report on standard output plus an overall verdict; `--format json` emits every check with its class, status, and message. Exit code is 0 when everything passes or only inert checks are skipped, and non-zero when a hard check fails. Each failing check carries the four-part error shape from [exit codes](./exit-codes.md).

The child version floor is a **perishable fact**: the child is externally owned and changes on its own schedule. It is registered in [research tracking](./research-tracking.yaml), and the check is defensive — an unparsable version string is reported, not fatal.

## Further reading

- [`tracing`](https://docs.rs/tracing/) and [`tracing-subscriber`](https://docs.rs/tracing-subscriber/)
- [`no-color.org`](https://no-color.org/)
- [Command Line Interface Guidelines](https://clig.dev/)
