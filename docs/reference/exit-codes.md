# Exit codes

The wrapper's exit-code matrix, the error message shape, and the rule that governs the boundary between wrapper-originated and child-originated status.

This describes normative design. The crate is pre-implementation.

## Two regimes

There are exactly two, and confusing them is the classic wrapper bug.

**Before the child runs**, a failure is the wrapper's own. It exits with a code from the matrix below, drawn from the BSD `sysexits` convention, and writes a diagnostic to standard error.

**Once the child is running**, the wrapper's exit status is the child's, reproduced as faithfully as the process model allows. The wrapper contributes nothing. A wrapper that translates a child's exit code into its own scheme breaks every script that wraps it.

The boundary is the successful spawn. If the wrapper reached the point of having a live child, the child owns the answer.

## Wrapper matrix

Success is exit `0`. It carries no `err.kind`, writes nothing to standard error, and is **not a variant of the error type** — there is nothing for a closed enum to hold, and a variant that can never be constructed would have to be handled at every match anyway.

Every variant of the error type maps to exactly one code. There is **no catch-all arm** in the mapping: adding a variant without assigning it a code must fail the build. A test asserts the whole table; see [testing and quality](./testing-and-quality.md).

`err.kind` is a stable, machine-matchable identifier emitted with the diagnostic. It is part of the user-facing API: scripts match on it. Renaming one is a breaking change.

| `err.kind`           | Code | Name             | Fires when                                                                                                        |
| -------------------- | ---- | ---------------- | ----------------------------------------------------------------------------------------------------------------- |
| `Usage`              | 64   | `EX_USAGE`       | A wrapper flag or verb was malformed, or arguments conflict                                                       |
| `DataFormat`         | 65   | `EX_DATAERR`     | A configuration piece, manifest, or metadata file is syntactically valid but semantically wrong                   |
| `NoInput`            | 66   | `EX_NOINPUT`     | A file the user explicitly named does not exist or cannot be read                                                 |
| `Unavailable`        | 69   | `EX_UNAVAILABLE` | A required external facility is missing — no controlling terminal where one is required, no usable base directory |
| `Internal`           | 70   | `EX_SOFTWARE`    | An invariant the program controls was violated. A bug.                                                            |
| `OsError`            | 71   | `EX_OSERR`       | The operating system refused an operation the wrapper is entitled to — `fork` failed, a pipe could not be created |
| `Io`                 | 74   | `EX_IOERR`       | An I/O operation failed for a reason not covered more specifically                                                |
| `TempFail`           | 75   | `EX_TEMPFAIL`    | A condition a retry is expected to clear — a runtime lock held by a concurrent `claude-session`                   |
| `Auth`               | 77   | `EX_NOPERM`      | Credentials are missing, expired, or refused                                                                      |
| `Permission`         | 77   | `EX_NOPERM`      | A filesystem ownership or mode check failed                                                                       |
| `Config`             | 78   | `EX_CONFIG`      | Configuration is malformed, contains an unknown key, or is internally inconsistent                                |
| `ChildRecursion`     | 78   | `EX_CONFIG`      | Resolution produced the wrapper itself; see [process runtime](./process-runtime.md#recursion-guard)               |
| `ChildNotExecutable` | 126  | —                | The child binary was found but is not executable                                                                  |
| `ChildNotFound`      | 127  | —                | The child binary could not be resolved                                                                            |

Codes 126 and 127 are shell conventions rather than `sysexits` values, and they are used deliberately: a user who sees 127 already knows it means "not found", and a wrapper reporting a different code for that condition would be gratuitously surprising.

**`ChildRecursion` is `Config`, not 127.** Resolution succeeded — it produced the wrong binary. 127 would send the user hunting for an uninstalled `claude` when the actual fault is a `PATH` entry or a symlink pointing back at the wrapper, which is exactly the "found in a misconfigured state" that 78 names. Grouping it with the other two child failures would buy one greppable family at the cost of the remedy being wrong.

`Auth` and `Permission` share code 77. They are separate `err.kind` values because their fixes differ — re-authenticate versus repair a file mode — and the kind string is what a script should match on. The line between them is the **subject**, not the severity: `Auth` is about a credential's validity, `Permission` about a path's ownership or mode. A credential file with the wrong owner is `Permission`, because the credential may be perfectly valid and the fix is `chmod`.

`TempFail` is the only code that tells a caller to **try again**. Every other failure is a standing condition a retry reproduces, which is why a lock held by a concurrent run must not report as `Unavailable` — that code says the facility is missing, and the facility is present and busy.

`OsError` and `Internal` are both "the wrapper's fault" from a distance and must not be merged. `Internal` is a bug and belongs in an issue report; `OsError` means the machine refused — a process-table limit, memory pressure — and the wrapper is working correctly. `sysexits` draws exactly this line, reserving 70 for "non-operating system related errors as possible". A wrapper whose one job is `fork` and `exec` needs the distinction more than most programs do.

### Where this diverges from `sysexits`

Two mappings depart from the header's own guidance, deliberately. Both are recorded here so a reader checking against the source does not "correct" them:

- **`Permission` uses 77**, which the header says is "not intended for file system problems, which should use `NOINPUT` or `CANTCREAT`, but rather for higher level permissions." The wrapper's filesystem checks are higher-level permissions: `session-root-security` fails on a directory the wrapper can read perfectly well and **refuses** because its mode or owner is wrong. That is a policy decision, not a denied `open`. A genuine denied `open` on a user-named file is `NoInput` (66), as the header intends.
- **126 and 127 are shell conventions**, not `sysexits` values, for the reason given above the matrix.

Five codes are deliberately unused: `NoUser` (67) and `NoHost` (68) are mail-transport concepts; `OsFile` (72) is for critical system files, which the wrapper never reads; `CantCreat` (73) is for a _user-specified_ output file, and every file the wrapper creates is wrapper-owned, so those failures are `Io` (74). `Protocol` (76) is for a remote protocol exchange — a child that returns unparsable output is `DataFormat` (65), because the child is a local process and its output is data.

The parser exits `2` on a malformed invocation by default. That default is **overridden**: a malformed wrapper invocation exits `Usage` (64), because the matrix is the wrapper's single answer for what a code means and a parser-shaped exception to it would be one a script has to special-case. The stream the help text lands on is settled in [the CLI surface](./cli-surface.md#help).

### The one code outside the taxonomy

`1` is not in the matrix, because it is not a failure of the wrapper. It is emitted by `doctor --strict` alone, when the catalog completed and a soft check reported `warn` that the caller asked to treat as fatal. It carries no `err.kind` and no diagnostic, because nothing went wrong; see [ADR-0034](../decisions/ADR-0034-exit-one-when-doctor-strict-promotes-a-warning.md) and [the report](./logging-and-output.md#the-report).

Every other bare `1` is a bug.

### The code the language owns

`101` is Rust's panic status. It is **not** in the matrix and is never mapped to deliberately: reaching it means the wrapper panicked, which [coding conventions](./coding-conventions.md#panics) forbids outside test code. It is documented here because a script author who sees it deserves to know it means "report this", not "retry" or "fix your configuration".

It is deliberately **not** converted to `Internal` (70). A panic and a handled internal error need to stay distinguishable — 70 says the wrapper detected a broken invariant and reported it properly, 101 says it did not. Collapsing them would hide the second behind the first, and 101 is already the ecosystem-wide signal. A panic hook may improve the _message_; it must not change the status.

## Child status passthrough

Once the child is running:

| Child outcome                           | Wrapper's exit status                                                                    |
| --------------------------------------- | ---------------------------------------------------------------------------------------- |
| Exited with code N                      | Exits with N, unchanged, for all N in 0–255                                              |
| Killed by signal N                      | Reproduces the death: resets the signal to its default action and re-raises it on itself |
| Killed by signal N, re-raise impossible | Exits with 128 + N, clamped to 255                                                       |

Re-raising is preferred over exiting with `128 + N` because it is more faithful. A parent examining the wrapper's wait status sees genuine signal death rather than a normal exit that merely encodes one — and those are distinguishable through the wait-status macros. The `128 + N` form is the documented fallback for the case where re-raise cannot be arranged, and it is what a shell reports either way.

The wrapper's own codes overlap this range. That is unavoidable — 64 is a legal child exit code as well as `EX_USAGE` — and it is why the two regimes are documented as a boundary rather than a disjoint numbering. A caller that needs to distinguish them reads standard error: a wrapper-originated failure always writes a diagnostic carrying an `err.kind`, and a passed-through child status never does.

Post-flight failures do **not** change the exit status of a passthrough invocation. See [process runtime](./process-runtime.md).

## Error message shape

Every wrapper-originated diagnostic opens with a fixed, greppable prefix carrying the program name and the `err.kind`:

```text
claude-session: error[ChildNotExecutable]: <what>
```

This prefix is the discriminator the [passthrough rule](#child-status-passthrough) above depends on. "The wrapper writes an `err.kind` and the child never does" is only checkable if the kind has a stable rendered form — otherwise a caller facing exit `64` cannot tell a wrapper usage error from a child that happens to exit `64`, which is the exact ambiguity the two regimes create. The bracketed-kind form follows `rustc`'s `error[E0308]:`, and the program-name prefix is the long-standing Unix convention that makes a diagnostic attributable when several programs share a pipeline's standard error.

The prefix is present at every verbosity, including `--quiet`: suppressing it would suppress the one part a script reads. In `--json` mode the whole diagnostic is the [error document](./logging-and-output.md#machine-output) instead, and `kind` is a field.

After the prefix come four parts, in this order:

| Part      | Content                                                                                            |
| --------- | -------------------------------------------------------------------------------------------------- |
| **What**  | What failed, in the user's vocabulary. Not the name of the internal function.                      |
| **Where** | The specific path, key, flag, or account involved. Always the concrete value, never a placeholder. |
| **Why**   | The underlying cause, including the operating system's message where there is one.                 |
| **Hint**  | A concrete next action. Omitted only when there genuinely is none.                                 |

For example, in substance rather than exact wording: the child binary is not executable; at the resolved absolute path; because the file mode denies execute for the current user; try making it executable or set the override variable to a different binary.

Diagnostics go to standard error. Never to standard output; see [logging and output](./logging-and-output.md).

## Stability

The matrix is part of the user-facing API and is **append-only**. Three rules, and the difference between them matters ([ADR-0033](../decisions/ADR-0033-append-fresh-exit-codes.md)):

- **A code's meaning is permanent.** It is never reassigned, and never widened to cover a second, unrelated class. This is the rule that protects existing callers: reuse silently changes what a branch already in the field catches.
- **A new failure class takes a new `err.kind` and an unused number**, preferring the `sysexits` category that already names the condition. Adding a number breaks nothing, so append rather than force a poor fit onto a code already in the table.
- **An existing `err.kind` is never renamed and never remapped.**

Several kinds **may** share one code where `sysexits` gives them the same category — `Auth` and `Permission` both use 77. The code is the coarse category; `err.kind` is the precise one, and it is what a script matches on. The stable unit is therefore the `err.kind`-to-code mapping, not the code's exclusivity.

Consumers branch on `0` versus non-zero, or on a documented code. A consumer that enumerates the set and treats an unknown number as impossible is relying on something this page does not promise.

Changing an existing mapping requires a decision record superseding [ADR-0005](../decisions/ADR-0005-exit-code-taxonomy.md).

## Inspection verbs and assertion verbs

Read-only verbs split into two kinds, and the split decides the exit.

**Inspection** — `profile list`, `account list`, `account status`, `version`. These exit `0` when they ran, **whatever they found**. The state reported is data, not the verb's own outcome: `version` against an unresolvable child, or `account status` with nothing selected, is a produced answer. They exit non-zero only when **the wrapper itself** failed — it could not read the file it was asked to inspect, or could not resolve a base directory.

A verb that exits non-zero because the answer was unwelcome cannot be used in a conditional, and its caller ends up parsing prose to recover the distinction.

**Assertion** — `config` and `doctor`. These are asked whether something holds, so answering "no" with `0` would make them useless as a gate. `config` validates as part of reporting ([ADR-0049](../decisions/ADR-0049-collapse-config-inspection-into-one-verb.md)), which is why it sits here rather than with the inspection verbs despite also rendering data:

| Invocation                               | Exit | Why                                                                      |
| ---------------------------------------- | ---- | ------------------------------------------------------------------------ |
| `config`, structurally sound             | `0`  | The assertion holds                                                      |
| `config`, unknown-key warnings only      | `0`  | Warnings are advisory by design; see [configuration](./configuration.md) |
| `config`, stale generated settings       | `0`  | Reported state, and the next launch regenerates                          |
| `config`, structural or type defect      | code | `DataFormat` or `Config`, by which defect it was                         |
| `doctor`, soft check failing             | `0`  | A degraded optional feature does not stop the wrapper working            |
| `doctor`, soft check failing, `--strict` | `1`  | The caller moved the threshold                                           |
| `doctor`, hard check failing             | code | The wrapper genuinely cannot function                                    |

Both draw the same line in the same place: a defect the subject can still function with is advisory and exits `0`, one it cannot is fatal. `--strict` exists so a caller who disagrees about where that line sits can move it without the verb having to guess.

## Error architecture

The exit-code mapping is exhaustive because the error type is a closed enum. The layering that keeps it closed — typed per-layer errors converging on one application error, with a boundary error type used only at the outermost edge — is in [coding conventions](./coding-conventions.md) and [ADR-0008](../decisions/ADR-0008-layered-error-architecture.md).

## Further reading

- [`sysexits(3)`](https://man.freebsd.org/cgi/man.cgi?query=sysexits&sektion=3)
- [Exit codes with special meanings](https://tldp.org/LDP/abs/html/exitcodes.html)
- [`thiserror`](https://docs.rs/thiserror/) and [`anyhow`](https://docs.rs/anyhow/)
- [Error handling in Rust](https://burntsushi.net/rust-error-handling/)
