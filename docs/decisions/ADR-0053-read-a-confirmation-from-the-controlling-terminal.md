# ADR-0053: Read a confirmation from the controlling terminal

## Context and Problem Statement

[ADR-0021](./ADR-0021-fail-closed-without-a-terminal.md) makes a confirming verb fail closed when there is no terminal, but never says how "no terminal" is detected. `isatty(0)` and opening `/dev/tty` disagree on a common invocation: `something | claude-session account remove work` has a pipe on standard input while the controlling terminal is still there. One prompts; the other exits `Unavailable` without asking.

## Considered Options

- **`isatty(0)`** — prompt only when standard input is itself a terminal.
- **Open `/dev/tty`** read-write, and use that handle for the whole exchange.

## Decision Outcome

Chosen option: **open the controlling terminal** — a confirmation needs a person, not whatever occupies file descriptor 0, and the project already says so three times: [exit codes](../reference/exit-codes.md) scopes `Unavailable` to a missing controlling terminal, ADR-0021 repeats it, and [ADR-0027](./ADR-0027-ingest-secrets-only-from-stdin-or-a-terminal.md) has token paste reading the controlling terminal with echo disabled. Under `isatty(0)`, `account remove` and `account login --token` would disagree about what "no terminal" means inside one verb family.

The predicate is the `open` itself: it succeeds and the verb prompts; it fails and the verb exits `Unavailable` before any side effect. A piped invocation prompts. Git prompts this way and never falls back to standard input; sudo reads its password from the terminal, with an explicit flag as the opt-out.

The counter-convention is real — clig.dev and `gh` gate prompting on standard input being a terminal. That fits a tool whose prompt competes with piped data, as `gh pr create < body.txt` does. No confirming verb here reads standard input, so a pipe carries no claim on the answer.

The prompt is therefore written to the terminal rather than to standard error, which [logging and output](../reference/logging-and-output.md#the-stream-contract) carries as the one exception to its stream table.

## Consequences

- Good: one predicate covers confirmation and token paste, so the verb family cannot disagree with itself.
- Good: `2>/dev/null` cannot swallow a prompt, and a piped invocation still gets one.
- Bad: it diverges from a widely cited guideline, so the reason has to be written down — this record.
- Bad: a test inherits the terminal unless it detaches, so a naive one hangs locally and passes in CI.

## Status

Accepted
