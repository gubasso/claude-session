# ADR-0021: Fail closed when a confirmation has no terminal

## Context and Problem Statement

Confirming verbs may run from scripts, pipelines, or containers where standard input is not a terminal. An unread prompt hangs; skipping it can destroy credentials.

## Considered Options

- Treat the absence of a terminal as consent and proceed.
- Claim a top-level `--yes` flag alongside the wrapper's other flags.
- Fail closed, with `--yes` accepted by the confirming verbs only.

## Decision Outcome

Chosen option: fail closed, with a verb-level `--yes` — absence of a terminal is absence of a user, not agreement from one, and reading it as consent makes a destructive verb silent under a pipe.

With no terminal and no escape, a confirming verb fails before side effects with `Unavailable`. `--yes` is the escape when a yes/no answer is the whole interaction. Browser login has no flag substitute; token mode is its headless path.

`--yes` is verb-level. A top-level flag permanently subtracts a spelling from the child ([ADR-0003](./ADR-0003-reserve-a-small-wrapper-cli-surface.md)); a verb-level flag is parsed only after the wrapper verb is chosen. No `--non-interactive` is added because detection already provides that behavior. See [the CLI surface](../reference/cli-surface.md).

## Consequences

- Good: no invocation hangs on an unread prompt, and no script is ever silently destructive.
- Bad: a verb-level flag does not appear in top-level help, so users must read the verb's help to find it.
- Bad: the rule is applied per verb rather than enforced centrally, so a new confirming verb can forget it and no lint will notice.

## Status

Accepted

Amended by [ADR-0027](./ADR-0027-ingest-secrets-only-from-stdin-or-a-terminal.md) and [ADR-0030](./ADR-0030-use-account-login-for-wrapper-authentication.md) for token terminology, and by [ADR-0053](./ADR-0053-read-a-confirmation-from-the-controlling-terminal.md) for the controlling-terminal predicate.
