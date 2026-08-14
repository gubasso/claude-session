# 003 — Native child exec

## Goal

Hand the child a correct argument vector and environment, then become it.

## Appetite

2 implementation sessions.

## Core

Resolution cannot recurse, the child's status and terminal behaviour are native because one process is left, and wrapper internals do not leak.

## In scope

- The exec, replacing spawn-and-wait, with the log flush ordered before it.
- The composed-settings argv prefix and the account configuration directory.
- The inherited proxy seam and a byte-recording end-to-end fixture.

## Out of scope

- Authentication mode selection, settings composition, and proxy request manipulation.
- Any signal handling, wait-status mapping, or work after the child.

## Governed by

- [AGENTS.md](../../../../AGENTS.md)
- [ADR-0084](../../../decisions/ADR-0084-exec-the-child-instead-of-supervising-it.md)
- [Wrapper model](../../../explanation/wrapper-model.md)
- [Testing strategy](../../../explanation/testing-strategy.md)
- [CLI surface](../../../reference/cli-surface.md)
- [Process runtime](../../../reference/process-runtime.md)
- [Exit codes](../../../reference/exit-codes.md)
- [Testing and quality](../../../reference/testing-and-quality.md)

## Acceptance

- When the child is resolved, the wrapper shall honor the documented ladder and both recursion guards.
- When the child runs, the wrapper shall have been replaced by it, so its exit code and signal death are the child's own.
- If the exec fails, then the wrapper shall report the documented class for the refusing errno rather than a child status. -> child_resolution::exec_failure_errno_is_classified
- When a profile is selected, the wrapper shall prepend the composed settings pair and keep user argv as an untouched suffix. -> passthrough::golden_argv_prefixes_the_settings_pair_before_an_untouched_suffix
- When the child environment is built, the wrapper shall scrub its namespace, restore only the reentry marker, set the session configuration directory and the account credential store, and preserve unrelated raw environment bytes. -> passthrough::a_selected_account_injects_its_config_directory_and_the_marker_alone
- While a log record is buffered, the wrapper shall flush the sink before replacing its image. -> tests::the_flush_precedes_the_launch

## Rabbit holes

- Signal forwarding, job control, and status mapping; escape: the exec owes none of them, and [ADR-0004](../../../decisions/ADR-0004-spawn-and-wait-child-supervision.md) is superseded rather than pending.
- Full account and profile resolution; escape: consume typed values from their owning slices.

## Done when

Targeted resolution, exec-failure, exit-status, environment, and byte-recording integration checks pass under `cargo nextest`.

## Revisions

None.
