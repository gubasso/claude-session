# ADR-0051: Let every surface element discriminate

## Context and Problem Statement

[ADR-0049](./ADR-0049-collapse-config-inspection-into-one-verb.md) collapsed `config`'s six subcommands into one verb, but recorded the outcome rather than the reason, so the next case had to be argued from scratch. Two arrived at once: `profile list` was a namespace with exactly one child, and `--profile` was declared `global = true`, reaching `version`, `completion`, `man`, and `help`, none of which read it.

## Considered Options

- Decide each case on its merits, as ADR-0049 did.
- Ban namespaces outright, flattening every verb.
- Name the shared rule: an element earns its place by discriminating between alternatives.

## Decision Outcome

Chosen option: name the shared rule. A subcommand distinguishes itself from its siblings; a flag distinguishes one invocation's behaviour from another's. An element with nothing to distinguish is a token the user must type to say nothing, and it is a contract that must be specified, tested, and kept working forever.

Two corollaries follow mechanically. A namespace verb with one subcommand collapses to the bare verb, so `profile list` becomes `profile`. A flag is declared on the invocations that act on it and nowhere else, so `--profile` is declared on the bare launch and on `config`, matching how [ADR-0024](./ADR-0024-machine-output-is-a-per-verb-flag.md) already scopes `--json`.

This is the mechanism behind [ADR-0048](./ADR-0048-build-for-a-present-need.md)'s YAGNI rule rather than a second rule beside it: YAGNI asks whether a surface answers a present need, and this asks what the surface would distinguish if it did.

Banning namespaces was rejected: `account` has four subcommands that genuinely discriminate, and flattening them would put four bare verbs in the child's argument space.

## Consequences

- Good: the next case is decided by a test rather than by re-argument.
- Good: `--help` for a verb lists only flags that verb reads.
- Bad: adding a second subcommand to a collapsed verb reintroduces the namespace, which is a breaking change.
- Bad: "discriminates" is a judgement, so review enforces it, as with ADR-0048.

## Status

Accepted

Generalizes [ADR-0049](./ADR-0049-collapse-config-inspection-into-one-verb.md).
