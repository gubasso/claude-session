# ADR-0052: Require an explicit subcommand for a namespace verb

## Context and Problem Statement

[ADR-0051](./ADR-0051-let-every-surface-element-discriminate.md) kept `account` a namespace because its four subcommands discriminate, but left the subcommand position's failure cases unspecified: what bare `account` does, and what an unrecognized subcommand does. `account` is the wrapper's only namespace verb, so these are the last two open questions in the verb grammar.

## Considered Options

- **Imply an inspection subcommand**, so bare `account` means `account status`, as `git remote` and `git submodule` do.
- **Treat the bare verb as a help request** — the verb's help on standard output, exit `0`.
- **Require the subcommand**, printing the verb's help as a diagnostic.

## Decision Outcome

Chosen option: **require the subcommand** — the bare verb satisfies no invocation, so it is malformed, and [the help rule](../reference/cli-surface.md#help) already routes help printed for a malformed invocation to standard error at `Usage`. The parser's full verb help is printed rather than a synopsis line, since the reader's next action is choosing a subcommand.

Implying a subcommand was rejected on three grounds. It leaves two spellings for one behaviour, which is the element ADR-0051 exists to reject. `account status [name]` takes an optional account identifier, and identifiers are `[a-z0-9_-]{1,32}` under [XDG storage](../reference/xdg-storage.md#group-identifiers), so `account list` becomes ambiguous with an account named `list`. And `status` with no selected account is already `Usage`, making it the one default that fails on a first run — `list` exits `0` there. Git added the pattern before 2010 and abandoned it: `git worktree` and `git bisect` exit `129` on a missing subcommand.

An unrecognized subcommand is the same failure and takes the same code and stream. It **does** carry a nearest-match suggestion, unlike a mistyped wrapper flag: [ADR-0043](./ADR-0043-match-wrapper-flags-by-exact-leading-spelling.md) forbids one because the wrapper has no model of the child's flags, and once a verb matches, [the pre-split](../reference/cli-surface.md#parser-shape) hands the remainder to the parser, whose subcommand set is closed and wholly wrapper-owned.

## Consequences

- Good: no reserved identifiers, and no bare verb whose meaning is breaking to change later.
- Good: the rule is the existing help rule applied, not a new one.
- Bad: `account` alone costs an error where a listing would have served.

## Status

Accepted
