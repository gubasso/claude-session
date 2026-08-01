# ADR-0056: Classify a failed spawn by its cause

## Context and Problem Statement

Child resolution checks that the path exists and is executable, then spawns. `access(2)` warns that this check-then-use pattern is racy, and the spawn step routed _every_ failure to `OsError` (71). So a child deleted or `chmod -x`'d between the check and the spawn reported 71, contradicting the promise that not-found and not-executable stay distinct all the way to the exit code. `ENOEXEC` — found, exec bit set, unloadable — had no classification at all.

## Considered Options

- Classify the spawn's `errno` into the same kinds the pre-flight check produces
- Keep `OsError` for every spawn failure and treat the pre-flight check as authoritative
- Close the race with `fexecve`, holding a descriptor from check to spawn

## Decision Outcome

Chosen option: **classify the spawn's `errno`** — the spawn is the authoritative attempt, so its failure should name the same condition the pre-flight check would have named.

The pre-flight check is therefore **advisory**: it exists to produce a good diagnostic early, not to guarantee the spawn succeeds. `ENOEXEC` maps to `ChildNotExecutable`, matching POSIX's definition of exit 126 as "found, but not an executable utility". The table is in [process runtime](../reference/process-runtime.md#spawn-and-wait).

`fexecve` is rejected: it complicates interpreted children, does not fit `std::process::Command`, and buys atomicity this unprivileged wrapper does not need — the race window's only consequence is which correct-shaped error the user reads.

## Consequences

- Good: `OsError` keeps the scope it claims — the machine refused, and the wrapper is working correctly.
- Good: a child removed mid-run reports 127, the code whose remedy is right.
- Bad: `ENOEXEC` is absent from Rust's `io::ErrorKind` table, so it must be read through `raw_os_error`, which is a version-sensitive detail rather than a stable API.
- Bad: two code paths can produce `ChildNotFound`, and a test must cover both.

## Status

Accepted
