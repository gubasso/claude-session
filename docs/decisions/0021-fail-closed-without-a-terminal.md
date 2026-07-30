# ADR-0021: Fail closed when a confirmation has no terminal

## Context and Problem Statement

Two wrapper verbs need the user's agreement: `account remove` destroys credentials, and `account add` runs a browser login. Both can be invoked from a script, a pipeline, or a container where standard input is not a terminal. A prompt written to a stream nobody reads hangs forever; skipping the prompt silently destroys something. The wrapper needs one rule for both.

## Considered Options

- Treat the absence of a terminal as consent and proceed.
- Claim a top-level `--yes` flag alongside the wrapper's other flags.
- Fail closed, with `--yes` accepted by the confirming verbs only.

## Decision Outcome

Chosen option: **fail closed, with a verb-level `--yes`** — absence of a terminal is absence of a user, not agreement from one, and reading it as consent makes a destructive verb silent under a pipe.

With no terminal and no escape, a confirming verb fails **before any side effect**, exiting `Unavailable` — a kind that already covers a missing controlling terminal, so [exit codes](../reference/exit-codes.md) gains nothing. `--yes` is the escape where a yes/no answer is the whole interaction, as in `account remove`. Where the interaction is a browser login no flag can substitute, so `account add`'s headless escape is the API-token path instead.

`--yes` is scoped to the verb rather than to the top level. A top-level flag is intercepted before the passthrough split and is therefore subtracted from the child's reachable surface permanently ([ADR-0003](./0003-reserve-a-small-wrapper-cli-surface.md)); a verb-level flag is parsed inside an invocation the child never sees, so it costs nothing. No `--non-interactive` flag is added — detection already produces that behaviour, so the flag buys nothing. The per-verb table is in [the CLI surface](../reference/cli-surface.md).

## Consequences

- Good: no invocation hangs on an unread prompt, and no script is ever silently destructive.
- Good: the claimed top-level flag set is unchanged, so this is not a passthrough-contract change.
- Bad: a verb-level flag does not appear in top-level help, so users must read the verb's help to find it.
- Bad: the rule is applied per verb rather than enforced centrally, so a new confirming verb can forget it and no lint will notice.

## Status

Accepted

Amended by [ADR-0027](./0027-ingest-secrets-only-from-stdin-or-a-terminal.md) and [ADR-0030](./0030-use-account-login-for-wrapper-authentication.md): `account login --token [--stdin]` and long-lived subscription-token terminology replace the historical planned `account add` and API-token wording.
