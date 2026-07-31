# ADR-0043: Match wrapper flags by exact leading spelling

## Context and Problem Statement

[ADR-0003](./ADR-0003-reserve-a-small-wrapper-cli-surface.md) enumerates the claimed flags but never fixes how a token is recognized as one. Every relaxation of matching — prefix abbreviation, short bundling, case folding — is another way the pre-split silently eats a token meant for the child, which is the one failure [ADR-0002](./ADR-0002-verbatim-argv-passthrough.md) exists to prevent.

## Considered Options

- Exact, case-sensitive, unabbreviated, unbundled matching in leading position.
- Reuse the parser's default matching, which accepts bundling and attached short values.
- Move the whole surface behind a project-qualified prefix such as `--claude-session-*`.

## Decision Outcome

Chosen option: **exact leading spelling** — the narrowest rule that can be stated in one paragraph and tested exhaustively, and the only one that stays correct as the child grows flags.

A token is a wrapper flag only when it is byte-identical to a claimed spelling, appears before the first non-wrapper token, and is not preceded by `--`. No prefix abbreviation, no short-flag bundling, no case folding, no aliases beyond those enumerated. Value-taking flags accept `--flag=<value>` and `--flag <value>`; in the separated form a next token beginning with `-` is **not** consumed, so a missing value is a `Usage` error rather than a stolen child token.

Anything that is not an exact claimed spelling is child argv, forwarded verbatim, with no diagnostic and no suggestion — the wrapper has no model of the child's grammar and may not pretend to one.

## Consequences

- Good: the pre-split stays pure, total, and exhaustively testable over a closed spelling set.
- Good: a near-miss such as `--configg` reaches the child, where it belongs.
- Bad: a user typo in a wrapper flag surfaces as a child error, which reads as further away from its cause.
- Bad: `--verbose` cannot be shortened, and repeat verbosity costs the full spelling each time.
- Rejected: the `--claude-session-*` prefix buys collision-proofing that the audit in [ADR-0044](./ADR-0044-audit-wrapper-spellings-against-the-child-inventory.md) already provides, at a per-invocation cost [ADR-0003](./ADR-0003-reserve-a-small-wrapper-cli-surface.md) already declined.

See [the CLI surface](../reference/cli-surface.md#flag-spelling).

## Status

Accepted

Amends [ADR-0003](./ADR-0003-reserve-a-small-wrapper-cli-surface.md).
