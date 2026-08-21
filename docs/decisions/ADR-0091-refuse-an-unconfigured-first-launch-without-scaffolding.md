# ADR-0091: Refuse an unconfigured first launch without scaffolding

## Context and Problem Statement

Mandatory session binding leaves a fresh configuration tree without either required selection. The wrapper must give that first launch a deterministic outcome without choosing credentials or settings for the user.

## Considered Options

- Refuse with concrete recovery actions.
- Materialize starting account and profile state.
- Exempt a fresh tree from mandatory binding.

## Decision Outcome

Chosen option: refuse with concrete recovery actions — the wrapper names every missing axis before any session side effect.

The recovery hint names only what an installed binary can reach: `claude-session account login <name>`, spelled with a name because the verb refuses without one when no account is selected, and the resolved `profiles/<name>.yaml` destination the wrapper itself reads. It names no repository path. `docs/` is excluded from the published crate, so an example that exists in the checkout does not exist for an installed user, and a hint pointing there would fail [self-containment](../../AGENTS.md). The shipped examples remain online reading; the program's own surfaces are `--help`, `man`, and `doctor`.

The wrapper does not materialize a starting pair. Doing so would choose or write user authentication and configuration, reversing [ADR-0006](./ADR-0006-place-files-by-xdg-ownership.md) and [ADR-0015](./ADR-0015-retire-the-init-verb.md). A fresh-tree exception would make attribution depend on filesystem history.

## Consequences

- Good: first-launch failure is explicit, actionable, and free of session side effects; the log sink every invocation installs is not one.
- Good: account and profile ownership remain with the user.
- Bad: first use requires setup before the child can run.

## Status

Implemented

Enacted by [the mandatory binding gate](../reference/process-runtime.md#the-exec) and its integration evidence.
