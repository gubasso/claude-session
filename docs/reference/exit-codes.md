# Exit codes

The wrapper's exit-code matrix, the error message shape, and the rule that governs the boundary between wrapper-originated and child-originated status.

The wrapper matrix, passthrough boundary, `doctor --strict` promotion, account inspection, and both account operations are implemented, including `NoInput` for a named account that does not exist and `LockBusy` at the credential lock's acquisition deadline. Login failures use `Auth` or `Unavailable`, may carry `child_exit`, and never replace passthrough's native child status. Profile resolution, the `profile` inspection regime, and the `config` assertion regime are implemented, and every row of the profile-resolution table below is pinned by a test.

## Two regimes

There are exactly two, and confusing them is the classic wrapper bug.

Before the child runs, a failure is the wrapper's own. It exits with a code from the matrix below, drawn from the BSD `sysexits` convention, and writes a diagnostic to standard error.

Once the child is running, there is no wrapper. The exec replaced it, so the status the caller sees is the child's own rather than a reproduction of it, and the wrapper contributes nothing because it no longer exists. A wrapper that translated a child's exit code into its own scheme would break every script that wraps it; this one cannot, which is the point of [ADR-0084](../decisions/ADR-0084-exec-the-child-instead-of-supervising-it.md).

The boundary is the successful exec of a passthrough launch. Everything on the wrapper's side of it is in the matrix; everything after it belongs to `claude`.

A verb that spawns the child as a subroutine — `account login`, `doctor`, `version` — is not that case. It asked the child a question and reports its own conclusion, so it keeps its own code from the matrix and its own standard output, and attributes the child instead ([ADR-0068](../decisions/ADR-0068-spawn-the-child-as-a-subroutine.md)).

Keeping the code is not softening the answer. Where such a verb composes with the child's own report, the child's level crosses unchanged — a child that reports failure fails the run — and the verb answers `Unavailable` (69), reporting the child's own code as data beside it ([ADR-0085](../decisions/ADR-0085-carry-the-child-report-level-into-the-verdict.md)). The matrix stays closed and the child's status stays legible; [`doctor`](./doctor.md#the-three-levels) owns what the composed report looks like.

## Wrapper matrix

Success is exit `0`. It carries no `err.kind`, writes nothing to standard error, and is not a variant of the error type — there is nothing for a closed enum to hold, and a variant that can never be constructed would have to be handled at every match anyway.

Every variant of the error type maps to exactly one code. There is no catch-all arm in the mapping: adding a variant without assigning it a code must fail the build. A test asserts the whole table; see [testing and quality](./testing-and-quality.md).

`err.kind` is a stable, machine-matchable identifier emitted with the diagnostic. It is part of the user-facing API: scripts match on it. Renaming one is a breaking change.

| `err.kind`           | Code | Name             | Fires when                                                                                                        |
| -------------------- | ---- | ---------------- | ----------------------------------------------------------------------------------------------------------------- |
| `Usage`              | 64   | `EX_USAGE`       | A wrapper flag or verb was malformed, or arguments conflict                                                       |
| `DataFormat`         | 65   | `EX_DATAERR`     | A configuration piece, profile, or metadata file is syntactically valid but semantically wrong                    |
| `NoInput`            | 66   | `EX_NOINPUT`     | A file the user explicitly named does not exist or cannot be read                                                 |
| `Unavailable`        | 69   | `EX_UNAVAILABLE` | A required external facility is missing — no controlling terminal where one is required, no usable base directory |
| `Internal`           | 70   | `EX_SOFTWARE`    | An invariant the program controls was violated. A bug.                                                            |
| `OsError`            | 71   | `EX_OSERR`       | The operating system refused an operation the wrapper is entitled to — `fork` failed, a pipe could not be created |
| `Io`                 | 74   | `EX_IOERR`       | An I/O operation failed for a reason not covered more specifically                                                |
| `LockBusy`           | 75   | `EX_TEMPFAIL`    | A write lock was still held by another run when the acquisition deadline expired                                  |
| `Auth`               | 77   | `EX_NOPERM`      | Credentials are missing, expired, or refused                                                                      |
| `Permission`         | 77   | `EX_NOPERM`      | A filesystem ownership, type, or symlink check failed, or a mode could not be corrected                           |
| `Config`             | 78   | `EX_CONFIG`      | Configuration is malformed, contains an unknown key, or is internally inconsistent                                |
| `ChildRecursion`     | 78   | `EX_CONFIG`      | Resolution produced the wrapper itself; see [process runtime](./process-runtime.md#recursion-guard)               |
| `ChildNotExecutable` | 126  | —                | The child binary was found but is not executable                                                                  |
| `ChildNotFound`      | 127  | —                | The child binary could not be resolved                                                                            |

Codes 126 and 127 are shell conventions rather than `sysexits` values, and they are used deliberately: a user who sees 127 already knows it means "not found", and a wrapper reporting a different code for that condition would be gratuitously surprising.

`ChildRecursion` is `Config`, not 127. Resolution succeeded — it produced the wrong binary. 127 would send the user hunting for an uninstalled `claude` when the actual fault is a `PATH` entry or a symlink pointing back at the wrapper, which is exactly the "found in a misconfigured state" that 78 names. Grouping it with the other two child failures would buy one greppable family at the cost of the remedy being wrong.

`Auth` and `Permission` share code 77. They are separate `err.kind` values because their fixes differ — re-authenticate versus repair a path — and the kind string is what a script should match on. The line between them is the subject, not the severity: `Auth` is about a credential's validity, `Permission` about a path's ownership, type, or mode. A credential file with the wrong owner is `Permission`, because the credential may be perfectly valid and what is wrong is where it sits.

`LockBusy` is the one code that tells a caller to try again. Every other failure in the matrix is a standing condition a retry reproduces. It fires only where the wrapper genuinely contends — the [credential scope](./xdg-storage.md#lock-scopes), whether the contending run is minting a token ([ADR-0060](../decisions/ADR-0060-lock-the-writes-that-are-not-derivable.md)) or removing the account ([ADR-0069](../decisions/ADR-0069-destroy-the-credential-lock-with-its-scope.md)) — and never on a lock left behind by a killed run, because the kernel releases those.

`OsError` and `Internal` are both "the wrapper's fault" from a distance and must not be merged. `Internal` is a bug and belongs in an issue report; `OsError` means the machine refused — a process-table limit, memory pressure — and the wrapper is working correctly. `sysexits` draws exactly this line, reserving 70 for "non-operating system related errors as possible". A wrapper whose one job is `fork` and `exec` needs the distinction more than most programs do.

### Where this diverges from `sysexits`

Two mappings depart from the header's own guidance, deliberately. Both are recorded here so a reader checking against the source does not "correct" them:

- `Permission` uses 77, which the header says is "not intended for file system problems, which should use `NOINPUT` or `CANTCREAT`, but rather for higher level permissions." The wrapper's filesystem checks are higher-level permissions: `storage-paths-owned` fails on a directory the wrapper can read perfectly well and refuses because its owner is wrong. That is a policy decision, not a denied `open`. A genuine denied `open` on a user-named file is `NoInput` (66), as the header intends.
- 126 and 127 are shell conventions, not `sysexits` values, for the reason given above the matrix.

Five codes are deliberately unused: `NoUser` (67) and `NoHost` (68) are mail-transport concepts; `OsFile` (72) is for critical system files, which the wrapper never reads; `CantCreat` (73) is for a user-specified output file, and every file the wrapper creates is wrapper-owned, so those failures are `Io` (74). `Protocol` (76) is for a remote protocol exchange — a child that returns unparsable output is `DataFormat` (65), because the child is a local process and its output is data.

The parser exits `2` on a malformed invocation by default. That default is overridden: a malformed wrapper invocation exits `Usage` (64), because the matrix is the wrapper's single answer for what a code means and a parser-shaped exception to it would be one a script has to special-case. The parser's invalid-UTF-8 rejection, which a wrapper flag requiring text produces, arrives the same way. The stream the help text lands on is settled in [the CLI surface](./cli-surface.md#help).

### The one code outside the taxonomy

`1` is not in the matrix, because it is not a failure of the wrapper. It is emitted by `doctor --strict` alone, when the catalog completed and a soft check reported `warn` that the caller asked to treat as fatal. It carries no `err.kind` and no diagnostic, because nothing went wrong; see [ADR-0034](../decisions/ADR-0034-exit-one-when-doctor-strict-promotes-a-warning.md) and [the report](./doctor.md#the-report).

Every other bare `1` is a bug.

### The code the language owns

`101` is Rust's panic status. It is not in the matrix and is never mapped to deliberately: reaching it means the wrapper panicked, which [coding conventions](./coding-conventions.md#panics) forbids outside test code. It is documented here because a script author who sees it deserves to know it means "report this", not "retry" or "fix your configuration".

It is deliberately not converted to `Internal` (70). A panic and a handled internal error need to stay distinguishable — 70 says the wrapper detected a broken invariant and reported it properly, 101 says it did not. Collapsing them would hide the second behind the first, and 101 is already the ecosystem-wide signal. A panic hook may improve the message; it must not change the status.

### Statuses the wrapper never mints

A status already owned by something else is never assigned to a wrapper failure, however well it seems to fit. Each is settled elsewhere on this page; this is the single place to look them up:

| Status                       | Owner                                                                                                    |
| ---------------------------- | -------------------------------------------------------------------------------------------------------- |
| `1`                          | `doctor --strict` alone, and nothing else — [above](#the-one-code-outside-the-taxonomy)                  |
| `2`                          | The parser's default, deliberately overridden to `Usage` — [above](#where-this-diverges-from-sysexits)   |
| `67`, `68`, `72`, `73`, `76` | `sysexits` categories with no condition in this program — [above](#where-this-diverges-from-sysexits)    |
| `101`                        | The Rust runtime — [above](#the-code-the-language-owns)                                                  |
| `129`–`165`                  | Signal death, which is the child's own and never a wrapper encoding — [below](#child-status-passthrough) |

`126` and `127` are the exception that proves the rule: they are shell conventions and the wrapper claims them on purpose, for the reason given above the matrix. Every status in the table is reachable from a wrapper run — but only as the child's, or as the language's, never as a wrapper error's.

## Child status passthrough

Once the child is running there is nothing left to specify. The exec left one process, so an exit code is the child's exit code and signal death is the child's own death, with a wait status that a parent's `WIFSIGNALED` reads as genuine rather than as a normal exit encoding `128 + N`. No mapping exists to get wrong, and no `128 + N` fallback exists to reach for.

The wrapper's own codes overlap the child's range. That is unavoidable — 64 is a legal child exit code as well as `EX_USAGE` — and it is why the two regimes are documented as a boundary rather than a disjoint numbering. A caller that needs to distinguish them reads standard error: a wrapper-originated failure always writes a diagnostic carrying an `err.kind`, and a child's own status never does.

There is no post-flight, so nothing can change a status after the launch. See [process runtime](./process-runtime.md#the-exec).

## Error message shape

Every wrapper-originated diagnostic opens with a fixed, greppable prefix carrying the program name and the `err.kind`:

```text
claude-session: error[ChildNotExecutable]: <what>
```

This prefix is the discriminator the [passthrough rule](#child-status-passthrough) above depends on. "The wrapper writes an `err.kind` and the child never does" is only checkable if the kind has a stable rendered form — otherwise a caller facing exit `64` cannot tell a wrapper usage error from a child that happens to exit `64`, which is the exact ambiguity the two regimes create. The bracketed-kind form follows `rustc`'s `error[E0308]:`, and the program-name prefix is the long-standing Unix convention that makes a diagnostic attributable when several programs share a pipeline's standard error.

The prefix is present at every verbosity, including `--quiet`: suppressing it would suppress the one part a script reads. In `--json` mode the whole diagnostic is the [error document](./logging-and-output.md#machine-output) instead, and `kind` is a field.

After the prefix come four parts, in this order:

| Part  | Content                                                                                                                                                                              |
| ----- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| What  | What failed, in the user's vocabulary. Not the name of the internal function.                                                                                                        |
| Where | The specific path, key, flag, or account involved. Always the concrete value, never a placeholder.                                                                                   |
| Why   | The underlying cause, including the operating system's message where there is one. Where a [subroutine child](#two-regimes) produced it, Why opens with that command and its status. |
| Hint  | A concrete next action. Omitted only when there genuinely is none.                                                                                                                   |

For example, in substance rather than exact wording: the child binary is not executable; at the resolved absolute path; because the file mode denies execute for the current user; try making it executable or set the override variable to a different binary.

Where the failure came from a catalog check, Hint is that check's [remediation](./doctor.md#remediations) template, verbatim. This is what makes the four-part shape and the one-remediation-per-check rule ([ADR-0018](../decisions/ADR-0018-one-probe-set-with-stable-check-ids.md)) one rule rather than two: a user who hits a command guard and a user who ran `doctor` read the same sentence.

Diagnostics go to standard error. Never to standard output; see [logging and output](./logging-and-output.md).

## Stability

The matrix is part of the user-facing API and is append-only. Three rules, and the difference between them matters ([ADR-0033](../decisions/ADR-0033-append-fresh-exit-codes.md)):

- A code's meaning is permanent. It is never reassigned, and never widened to cover a second, unrelated class. This is the rule that protects existing callers: reuse silently changes what a branch already in the field catches.
- A new failure class takes a new `err.kind` and an unused number, preferring the `sysexits` category that already names the condition. Adding a number breaks nothing, so append rather than force a poor fit onto a code already in the table.
- An existing `err.kind` is never renamed and never remapped.

Several kinds may share one code where `sysexits` gives them the same category — `Auth` and `Permission` both use 77. The code is the coarse category; `err.kind` is the precise one, and it is what a script matches on. The stable unit is therefore the `err.kind`-to-code mapping, not the code's exclusivity.

Consumers branch on `0` versus non-zero, or on a documented code. A consumer that enumerates the set and treats an unknown number as impossible is relying on something this page does not promise.

Changing an existing mapping requires a decision record superseding [ADR-0005](../decisions/ADR-0005-exit-code-taxonomy.md).

## Exit regimes by verb

Every invocation belongs to exactly one of four regimes, and the regime decides the exit. The first is the boundary [above](#two-regimes); the other three are all wrapper-owned, and they differ in what the exit is about.

| Regime      | Invocations                                                    | The exit answers                   |
| ----------- | -------------------------------------------------------------- | ---------------------------------- |
| Passthrough | The bare launch                                                | Nothing — it is the child's status |
| Inspection  | `profile`, `account list`, `account status`, `version`         | Did the verb run?                  |
| Assertion   | `config`, `doctor`                                             | Does the property hold?            |
| Operation   | `account login`, `account remove`, `completion`, `man`, `help` | Did the operation complete?        |

Inspection — `profile`, `account list`, `account status`, `version`. These exit `0` when they ran, whatever they found. The state reported is data, not the verb's own outcome: `version` against an unresolvable child, or `account status` with nothing selected, is a produced answer. They exit non-zero only when the wrapper itself failed — it could not read the file it was asked to inspect, or could not resolve a base directory.

A verb that exits non-zero because the answer was unwelcome cannot be used in a conditional, and its caller ends up parsing prose to recover the distinction.

Assertion — `config` and `doctor`. These are asked whether something holds, so answering "no" with `0` would make them useless as a gate. `config` validates as part of reporting ([ADR-0049](../decisions/ADR-0049-collapse-config-inspection-into-one-verb.md)), which is why it sits here rather than with the inspection verbs despite also rendering data:

| Invocation                               | Exit | Why                                                           |
| ---------------------------------------- | ---- | ------------------------------------------------------------- |
| `config`, structurally sound             | `0`  | The assertion holds                                           |
| `config`, entry not yet written          | `0`  | Reported state, and the next launch writes it                 |
| `config`, structural or type defect      | code | `DataFormat` or `Config`, by which defect it was              |
| `doctor`, soft check failing             | `0`  | A degraded optional feature does not stop the wrapper working |
| `doctor`, soft check failing, `--strict` | `1`  | The caller moved the threshold                                |
| `doctor`, hard check failing             | code | The wrapper genuinely cannot function                         |

Both draw the same line in the same place: a defect the subject can still function with is advisory and exits `0`, one it cannot is fatal. `--strict` exists so a caller who disagrees about where that line sits can move it without the verb having to guess.

Operation — `account login`, `account remove`, `completion`, `man`, `help`. These change something or produce something rather than reporting on state, so neither of the two rules above applies: there is no finding to exit `0` over and no property to assert. They exit `0` when the operation completed and a matrix code when the wrapper's own handling failed. A `remove` the user declined exits `0` because the exchange completed as designed, not because it inspected anything ([the CLI surface](./cli-surface.md#the-exchange) owns the exchange). Where one of them ran the child as a subroutine, [ADR-0068](../decisions/ADR-0068-spawn-the-child-as-a-subroutine.md) still governs the attribution.

### Resolving a profile name

`--profile` is not a verb, so its failures land on whichever invocation declared it — the bare launch or `config` ([configuration](./configuration.md#selecting-the-active-profile)). `profile` itself takes no argument and cannot reach them:

| Condition                                                | Exit | Why                                                           |
| -------------------------------------------------------- | ---- | ------------------------------------------------------------- |
| No name resolved from any layer                          | `0`  | Nothing was asked for, so nothing is composed                 |
| A resolved name has no `profiles/<name>.yaml`            | `66` | `NoInput`, whichever layer named it                           |
| A resolved profile names a piece that does not exist     | `66` | `NoInput`; the error names the profile and the path           |
| A resolved profile is malformed or has an empty `layers` | `65` | `DataFormat`; it parsed and is semantically wrong             |
| A resolved name fails the identifier rules               | `64` | `Usage`, whichever layer named it                             |
| A materialized entry disagrees with its recomputed key   | `65` | `DataFormat`; it is neither opened nor overwritten            |
| `profile` with no `profiles/` directory, or an empty one | `0`  | An empty list is the answer, not a failure — it is inspection |

## Error architecture

The exit-code mapping is exhaustive because the error type is a closed enum. The layering that keeps it closed — typed per-layer errors converging on one application error, with a boundary error type used only at the outermost edge — is in [coding conventions](./coding-conventions.md) and [ADR-0008](../decisions/ADR-0008-layered-error-architecture.md).

## Further reading

- [`sysexits(3)`](https://man.freebsd.org/cgi/man.cgi?query=sysexits&sektion=3)
- [Exit codes with special meanings](https://tldp.org/LDP/abs/html/exitcodes.html)
- [`thiserror`](https://docs.rs/thiserror/) and [`anyhow`](https://docs.rs/anyhow/)
- [Error handling in Rust](https://burntsushi.net/rust-error-handling/)
