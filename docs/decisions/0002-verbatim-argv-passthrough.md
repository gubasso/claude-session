# ADR-0002: Forward argv verbatim to the child

## Context and Problem Statement

`claude-session` wraps the `claude` command, which is developed elsewhere and gains flags on its own schedule. Any argument handling the wrapper performs is a promise it must keep as the child changes. The project's first standing contract is that native passthrough must never break, so the wrapper needs a rule for what it does to the arguments it forwards.

## Considered Options

- Forward argv verbatim, intercepting only a documented denylist of the wrapper's own flags.
- Parse the child's full grammar and reconstruct the argument vector.
- Accept only an allowlist of understood child flags and reject the rest.

## Decision Outcome

Chosen option: **forward verbatim with a denylist** — a wrapper that understands its child's grammar breaks every time the child grows a flag, while a wrapper that understands almost nothing keeps working.

Verbatim is defined strictly. Order, bytes, and count are preserved; arguments are carried as OS strings and never round-tripped through UTF-8; an empty argument is a real argument and is never filtered; `--` is a hard sentinel after which nothing is interpreted. There is **no argv normalization step**, and adding one is a change to this decision rather than an implementation detail.

## Consequences

- Good: new child flags work through the wrapper the day they ship, with no wrapper release.
- Good: the forwarding logic is a small pure function, so the contract is directly testable — see the golden-argv table in [testing and quality](../reference/testing-and-quality.md).
- Good: non-UTF-8 arguments and paths survive, which UTF-8 round-tripping would corrupt.
- Bad: a wrapper flag makes the identically-named child flag unreachable except after `--`. This is why the denylist is small and long-form; see [ADR-0003](./0003-reserve-a-small-wrapper-cli-surface.md).
- Bad: the wrapper cannot validate or improve child arguments, so a child-side usage error surfaces from the child rather than earlier.
- Bad: a derive parser cannot express this alone — a leading unknown flag is not a positional and is rejected before external-subcommand handling applies — so argv must be pre-split before parsing. See [the CLI surface](../reference/cli-surface.md).

## Status

Accepted
