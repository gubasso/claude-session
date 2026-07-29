# Exit codes

The wrapper's exit-code matrix, the error message shape, and the rule that governs the boundary between wrapper-originated and child-originated status.

This describes normative design. The crate is pre-implementation.

## Two regimes

There are exactly two, and confusing them is the classic wrapper bug.

**Before the child runs**, a failure is the wrapper's own. It exits with a code from the matrix below, drawn from the BSD `sysexits` convention, and writes a diagnostic to standard error.

**Once the child is running**, the wrapper's exit status is the child's, reproduced as faithfully as the process model allows. The wrapper contributes nothing. A wrapper that translates a child's exit code into its own scheme breaks every script that wraps it.

The boundary is the successful spawn. If the wrapper reached the point of having a live child, the child owns the answer.

## Wrapper matrix

Every variant of the error type maps to exactly one code. There is **no catch-all arm** in the mapping: adding a variant without assigning it a code must fail the build. A test asserts the whole table; see [testing and quality](./testing-and-quality.md).

`err.kind` is a stable, machine-matchable identifier emitted with the diagnostic. It is part of the user-facing API: scripts match on it. Renaming one is a breaking change.

| `err.kind`           | Code | Name             | Fires when                                                                                                        |
| -------------------- | ---- | ---------------- | ----------------------------------------------------------------------------------------------------------------- |
| `Ok`                 | 0    | success          | The wrapper verb completed                                                                                        |
| `Usage`              | 64   | `EX_USAGE`       | A wrapper flag or verb was malformed, or arguments conflict                                                       |
| `DataFormat`         | 65   | `EX_DATAERR`     | A configuration piece, manifest, or metadata file is syntactically valid but semantically wrong                   |
| `NoInput`            | 66   | `EX_NOINPUT`     | A file the user explicitly named does not exist or cannot be read                                                 |
| `Unavailable`        | 69   | `EX_UNAVAILABLE` | A required external facility is missing — no controlling terminal where one is required, no usable base directory |
| `Internal`           | 70   | `EX_SOFTWARE`    | An invariant the program controls was violated. A bug.                                                            |
| `Io`                 | 74   | `EX_IOERR`       | An I/O operation failed for a reason not covered more specifically                                                |
| `Auth`               | 77   | `EX_NOPERM`      | Credentials are missing, expired, or refused; or a path failed its ownership check                                |
| `Permission`         | 77   | `EX_NOPERM`      | A filesystem permission or mode check failed                                                                      |
| `Config`             | 78   | `EX_CONFIG`      | Configuration is malformed, contains an unknown key, or is internally inconsistent                                |
| `ChildNotExecutable` | 126  | —                | The child binary was found but is not executable                                                                  |
| `ChildNotFound`      | 127  | —                | The child binary could not be resolved                                                                            |
| `ChildRecursion`     | 127  | —                | Resolution produced the wrapper itself; see [process runtime](./process-runtime.md)                               |

Codes 126 and 127 are shell conventions rather than `sysexits` values, and they are used deliberately: a user who sees 127 already knows it means "not found", and a wrapper reporting a different code for that condition would be gratuitously surprising.

`Auth` and `Permission` share code 77. They are separate `err.kind` values because their fixes differ — re-authenticate versus repair a file mode — and the kind string is what a script should match on.

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

Every wrapper-originated diagnostic has four parts, in this order:

| Part      | Content                                                                                            |
| --------- | -------------------------------------------------------------------------------------------------- |
| **What**  | What failed, in the user's vocabulary. Not the name of the internal function.                      |
| **Where** | The specific path, key, flag, or account involved. Always the concrete value, never a placeholder. |
| **Why**   | The underlying cause, including the operating system's message where there is one.                 |
| **Hint**  | A concrete next action. Omitted only when there genuinely is none.                                 |

For example, in substance rather than exact wording: the child binary is not executable; at the resolved absolute path; because the file mode denies execute for the current user; try making it executable or set the override variable to a different binary.

Diagnostics go to standard error. Never to standard output; see [logging and output](./logging-and-output.md).

## Stability

The matrix is part of the user-facing API and is **append-only**:

- A new failure class gets a new `err.kind` and a code drawn from the existing table.
- An existing `err.kind` is never renamed and never remapped to a different code.
- Several kinds **may** share one code where `sysexits` gives them the same category — `Auth` and `Permission` both use 77. The code is a coarse category; `err.kind` is the precise one, and it is what a script matches on.

The stable unit is therefore the `err.kind`-to-code mapping, not the code's exclusivity.

Changing an existing mapping requires a decision record superseding [ADR-0005](../decisions/0005-exit-code-taxonomy.md).

## Error architecture

The exit-code mapping is exhaustive because the error type is a closed enum. The layering that keeps it closed — typed per-layer errors converging on one application error, with a boundary error type used only at the outermost edge — is in [coding conventions](./coding-conventions.md) and [ADR-0008](../decisions/0008-layered-error-architecture.md).

## Further reading

- [`sysexits(3)`](https://man.freebsd.org/cgi/man.cgi?query=sysexits&sektion=3)
- [Exit codes with special meanings](https://tldp.org/LDP/abs/html/exitcodes.html)
- [`thiserror`](https://docs.rs/thiserror/) and [`anyhow`](https://docs.rs/anyhow/)
- [Error handling in Rust](https://burntsushi.net/rust-error-handling/)
