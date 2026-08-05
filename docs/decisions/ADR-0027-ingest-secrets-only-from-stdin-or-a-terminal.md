# ADR-0027: Ingest secrets only from stdin or a terminal

## Context and Problem Statement

A long-lived subscription token must enter the wrapper without appearing in process arguments or inherited state. The interactive mint command presents its result through child-owned terminal output whose format is not a wrapper contract.

## Considered Options

- Read one line from standard input or the controlling terminal with echo disabled.
- Accept a positional argument.
- Read an inherited environment variable.
- Add a wrapper-specific file flag.
- Scrape `claude setup-token` through a pseudo-terminal.

## Decision Outcome

Chosen option: standard input or the controlling terminal — `--stdin` reads one line without prompting; otherwise the wrapper reads one line from the controlling terminal with echo disabled.

For interactive minting, `setup-token` runs first with inherited standard streams. The user then pastes the result; the wrapper never parses the child's presentation.

## Consequences

- Good: token material stays out of argv, shell history, and ambient child environments.
- Good: because the wrapper never ingests a token from the environment, an inherited credential variable is unambiguously ambient — which is what lets [accounts](../reference/accounts.md) distinguish configured authentication from inherited authentication and warn about shadowing.
- Good: redirection already provides file-based automation without another flag.
- Bad: interactive use includes an explicit paste step.
- Bad: invocation without `--stdin` requires a controlling terminal under [ADR-0021](./ADR-0021-fail-closed-without-a-terminal.md).

The positional, environment, redundant `--from-file`, and pseudo-terminal scraping options are rejected. See [ADR-0002](./ADR-0002-verbatim-argv-passthrough.md).

## Status

Accepted
