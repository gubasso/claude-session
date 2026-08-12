# Logging and output

Every byte the wrapper writes, and where it goes. How a human-facing byte looks once it gets there is [presentation](./presentation.md)'s.

The passthrough, diagnostic, logging, composed `help`, `version`, and `doctor` output, all four `account` human and JSON reports, the precedence warnings, credential redaction, the `profile` and `config` human and JSON reports, and the raw `completion` and `man` results are implemented.

## The stream contract

Standard output carries the result and nothing else. Standard error carries everything else.

The test is whether a user could pipe the command into another program. If a byte would corrupt that pipe, it does not belong on standard output.

| Class                   | Stream                                  | Notes                                                                                                                                                                                                            |
| ----------------------- | --------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A wrapper verb's result | stdout                                  | Text, or JSON when the verb was given `--json`                                                                                                                                                                   |
| Progress and status     | stderr                                  | Never stdout, even when interactive                                                                                                                                                                              |
| A confirmation prompt   | The controlling terminal                | The one class not on a standard stream, so `2>/dev/null` cannot swallow it. Which verbs prompt, the predicate, and the exchange are in [the CLI surface](./cli-surface.md#confirmation-and-non-interactive-use). |
| Warnings                | stderr                                  |                                                                                                                                                                                                                  |
| Errors                  | stderr                                  | Four-part shape; see [exit codes](./exit-codes.md)                                                                                                                                                               |
| Log records             | Log file, optionally mirrored to stderr | See below                                                                                                                                                                                                        |
| The child's output      | Inherited                               | The wrapper never intercepts it. A [subroutine child](./exit-codes.md#two-regimes) under `--json` inherits stderr in place of stdout, which the verb's document has claimed.                                     |

During a passthrough invocation the wrapper writes nothing to standard output. Not a banner, not a progress line, not a "launching claude" notice. The child's standard output is the user's data stream and the wrapper is not entitled to a byte of it. Wrapper diagnostics during a passthrough go to standard error, where they are already interleaved with the child's.

Every terminal write goes through the output writer. The context carries one, and the entry point and the subscriber construct their own because they run outside a context — before one exists, and from inside the logging stack it would otherwise re-enter. Direct print macros are forbidden outside that writer and the entry point, and a lint enforces it; see [testing and quality](./testing-and-quality.md). One writer is what makes JSON mode, `--quiet`, and colour handling work uniformly instead of being reimplemented per command.

## Machine output

`--json` makes a wrapper verb emit a single JSON document on standard output. It is a mode, not a decoration: in JSON mode, no human-oriented text appears on standard output at all.

The flag is verb-level and every verb that produces data declares its own; there is no global `--format`. The reasoning is in [the CLI surface](./cli-surface.md#machine-output-is-not-on-this-table) and [ADR-0024](../decisions/ADR-0024-machine-output-is-a-per-verb-flag.md). One writer still renders every document, so the mode behaves identically across verbs even though the flag is declared per verb.

Errors in JSON mode still go to standard error, and are themselves a JSON object — the four parts of the [error shape](./exit-codes.md#error-message-shape) as fields, plus the `err.kind` a script branches on:

```json
{ "kind": "ChildNotExecutable", "what": "…", "where": "…", "why": "…", "hint": "…" }
```

This is the one document shape that is not the verb's to choose. A caller asking for JSON asked for it on both streams, and a failure is the case where falling back to prose is least useful.

It carries one optional field beyond the four parts: `child_exit`, an integer, present only when a [subroutine child](./exit-codes.md#two-regimes) produced the failure and it ran ([ADR-0068](../decisions/ADR-0068-spawn-the-child-as-a-subroutine.md)). Presence is how a script tells a wrapper-originated failure from one the child returned; absence means the wrapper's own handling failed.

Three rules apply to every document, whichever verb emits it:

- One document per invocation, and no envelope shared across verbs. Each verb's top-level object is its own shape, so a document can grow without an agreement every other verb has to honour. See [ADR-0032](../decisions/ADR-0032-give-each-verb-its-own-json-document.md).
- An absent optional field is omitted, never `null`. A consumer tests for presence, which is one branch rather than two.
- `schema_version` appears only where the document is itself a contract a script matches against — today that is [`doctor`](./doctor.md#the-report) alone, whose check ids are public API. Adding it everywhere would promise a versioning guarantee the other verbs do not make.

The error document above is the only shape fixed across every verb. Everything else — the top-level object, its fields, and whether `schema_version` appears at all — belongs to the verb that emits it and is specified on that verb's own page ([ADR-0032](../decisions/ADR-0032-give-each-verb-its-own-json-document.md)). That is the whole fixed-versus-variable boundary; an index of per-verb documents here would be a second home for every one of them.

## Composed output

Where the wrapper keeps a spelling the child also owns, the result carries both answers: the wrapper's own output first, complete on its own, then the child's bytes verbatim ([ADR-0079](../decisions/ADR-0079-compose-every-overlapping-surface-with-the-child.md)). Which spellings qualify is [the CLI surface](./cli-surface.md#when-the-child-owns-the-same-name)'s to say; this section owns what the seam looks like. It is the only case in which the wrapper's standard output carries bytes it did not produce.

One delimiter line separates the two, with a blank line on each side:

```text
--- claude doctor ---
```

Between the dashes is the exact command that produced what follows, so a reader can reproduce the second half on its own. The form is fixed: ASCII, and no padding to the terminal's width. Padding would make the result depend on the terminal it was printed into, which a golden test cannot pin and a pipe has no use for. The line is one of the surfaces [presentation](./presentation.md#coloured-surfaces) permits colour on, and carries nothing the command name does not already state.

The child's bytes pass through unchanged — not parsed, not re-indented, not re-wrapped, not summarized — and nothing follows them. The wrapper claimed the name in order to add its own answer, not to edit the child's. In human format that is usually literal: the wrapper writes its output and the delimiter, flushes, and the child inherits standard output, so no wrapper code ever holds those bytes.

`doctor` is the one surface that captures instead, because its verdict includes the child's status ([ADR-0085](../decisions/ADR-0085-carry-the-child-report-level-into-the-verdict.md)) and that status is only knowable once the child has exited. Capturing is what lets the wrapper's own bytes still come first. It changes nothing a reader sees: the captured bytes are replayed unchanged, standard error is replayed to standard error, and nothing follows them.

The composed section is part of the result, so `--quiet` does not suppress it and a redirected standard output receives it. JSON mode always captures, because the document has already claimed standard output: there is no delimiter, and the child's output is one opaque string field beside its status on the document the verb owns. When the child cannot be resolved or spawned, one line naming that condition takes the place of the section, and its absence does not change the exit status.

## Verbosity

| Invocation                      | Level              |
| ------------------------------- | ------------------ |
| `--quiet`                       | Errors only        |
| Default                         | Warnings and above |
| `--verbose`                     | Info and above     |
| `--verbose --verbose`           | Debug and above    |
| `--verbose --verbose --verbose` | Trace              |

Verbosity repeats by repeating the whole spelling, and `-vv` is not a wrapper token at all — it is forwarded to the child, like any other unclaimed argument. Both follow from exact, unbundled matching ([ADR-0043](../decisions/ADR-0043-match-wrapper-flags-by-exact-leading-spelling.md)). The spellings themselves, and why there is no `-v`, are in [the CLI surface](./cli-surface.md#wrapper-owned-flags).

A fourth `--verbose` is not an error; the level is clamped at trace. `--quiet` and `--verbose` together is a usage error, not a silent precedence rule.

`RUST_LOG` is honoured and, when set, overrides the flag-derived level. This is deliberate: the flag is the user's coarse control, and the environment variable is the developer's fine one. Only the level is read from it — the wrapper emits from one target, so per-module filtering would have nothing to select between, and a directive naming a module selects on its level alone. No `CLAUDE_SESSION_RS_LOG` variable is invented — reusing the ecosystem-standard name means existing knowledge transfers.

Verbosity governs the stderr mirror and nothing else. The file sink is fixed at `debug`, unconditionally: `--quiet`, `--verbose`, and `RUST_LOG` do not move it. A level that tracked the terminal would leave the file useless in exactly the case it exists for — a bug report from a user who ran with no flags. `trace` is deliberately not the fixed level, because trace is where child arguments are logged and always-on argument capture on a wrapper that forwards prompts is a standing redaction hazard.

## Log records

Exactly one subscriber is installed, from the entry point, before anything else runs. Installing a second is a bug.

The default sink is a file under the state base directory, written non-blocking and rotated. It is a file rather than the terminal because a wrapper's diagnostics interleaved with an interactive child's output are unreadable, and because the log's value is in being there after something went wrong.

The stderr mirror is driven by verbosity, and it is a human channel. In JSON mode it is off: standard error carries the error document there, and a mirrored record beside it would corrupt the one document shape a verb does not choose.

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

A value that would break either is quoted, and the characters that would break the line are escaped inside the quotes. A path with a space in it otherwise splits into two fields, and a message with a newline in it otherwise becomes two records — silently, and only for the values most worth reading.

Credentials, tokens, API keys, and helper output are never emitted, at any level or in any field. This covers human output, prompts, every `--json` document, logs, diagnostics, errors, and the diagnostic `where` clause. A concrete secret error location is its non-secret path, never its content.

The only secret-derived value permitted is `sha256[..8]` of a wrapper-owned OAuth token where [accounts](./accounts.md#token-lifecycle) requires it. Token prefixes and every hash or fingerprint of a child-owned credential are prohibited. Non-secret paths, modes, timestamps, and mode metadata remain reportable.

The child's arguments are not logged at default verbosity because a passed-through argument can contain a prompt, path, or secret. Trace-level argument logging must apply the same redaction rule.

## Further reading

- [`tracing`](https://docs.rs/tracing/) and [`tracing-subscriber`](https://docs.rs/tracing-subscriber/)
- [Command Line Interface Guidelines](https://clig.dev/)
