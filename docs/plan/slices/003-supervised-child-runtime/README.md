# Supervised child runtime

## Goal

Supervise the resolved child with faithful signals, status, argv, and isolated environment.

## Appetite

3 implementation sessions.

## Core

Resolution cannot recurse, child termination semantics remain native, and wrapper internals do not leak.

## In scope

- Version detail and child-resolution diagnostics.
- Account configuration and composed-settings argv prefix construction.
- Inherited proxy seam and a byte-recording end-to-end fixture.

## Out of scope

- Authentication mode selection, settings composition, and proxy request manipulation.
- Blanket signal forwarding or an `exec` path.

## Governed by

- [AGENTS.md](../../../../AGENTS.md)
- [Wrapper model](../../../explanation/wrapper-model.md)
- [Testing strategy](../../../explanation/testing-strategy.md)
- [CLI surface](../../../reference/cli-surface.md)
- [Process runtime](../../../reference/process-runtime.md)
- [Exit codes](../../../reference/exit-codes.md)
- [Testing and quality](../../../reference/testing-and-quality.md)

## Acceptance

- When the child is resolved, the wrapper shall honor the documented ladder and both recursion guards.
- When the child exits or dies by signal, the wrapper shall preserve the documented observable status and terminal behavior.
- If a terminal-broadcast signal reaches the process group, then the wrapper shall not double-deliver it.
- When the child environment is built, the wrapper shall scrub its namespace, restore only the reentry marker, and preserve unrelated raw environment bytes.
- When user argv is appended, the wrapper shall keep it as an untouched `OsString` suffix.

## Rabbit holes

- Proxy configuration surfaces; escape: inheritance is the complete seam.
- Full account and profile resolution; escape: consume typed values from their owning slices.

## Done when

Targeted resolution, signal, exit-status, environment, and byte-recording integration checks pass under `cargo nextest`.

## Revisions

None.
