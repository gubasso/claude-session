# ADR-0045: Compose doctor with the child report

## Context and Problem Statement

The wrapper claims `doctor` ([the CLI surface](../reference/cli-surface.md#wrapper-verbs)), and `claude` 2.1.220 ships `claude doctor` — measured 2026-07-31, the same collision class as `auth`, which [ADR-0030](./ADR-0030-use-account-login-for-wrapper-authentication.md) resolved by renaming. Renaming again would leave the wrapper without the one verb name every user reaches for when something is wrong, while shadowing outright hides a diagnostic the wrapper's own report cannot reproduce.

## Considered Options

- Run the wrapper's checks, then delegate to the child's `doctor` and report both.
- Rename the wrapper's verb, as `account` did for `auth`.
- Shadow the child's verb, reachable only through `--`.

## Decision Outcome

Chosen option: compose — the two reports answer different questions, and a user running `claude-session-rs doctor` wants both.

`claude-session doctor` emits its own report first, then spawns `claude doctor` and passes that output through unmodified under its own heading. The child's report is never parsed, summarized, or reformatted; its exit status is its own level, zero healthy and anything else carried across ([ADR-0085](./ADR-0085-carry-the-child-report-level-into-the-verdict.md)). Under `--json` the child's report is one opaque string field beside its status, so [ADR-0032](./ADR-0032-give-each-verb-its-own-json-document.md)'s schema never depends on the child's formatting.

This generalizes: a wrapper verb may keep a name the child also owns only when it runs the child's command as part of its own and reports the result, and the overlap is recorded with that reasoning in the CLI surface. Every other collision is renamed or reached through `--`.

## Consequences

- Good: one command diagnoses the whole stack, and neither report can silently disappear.
- Good: the exception is narrow and has a stated test — composition, not coincidence.
- Bad: `doctor` now spawns the child, so it is slower and can fail before the child resolves.
- Bad: the child's report format is outside the wrapper's control and its snapshot cannot be pinned.

## Status

Accepted

Amended by [ADR-0079](./ADR-0079-compose-every-overlapping-surface-with-the-child.md), which supplies the test this record generalized without stating: composition is available where the shared surface is read-only, and required there.

Amended by [ADR-0085](./ADR-0085-carry-the-child-report-level-into-the-verdict.md): the child's level enters the verdict, not a probe.
