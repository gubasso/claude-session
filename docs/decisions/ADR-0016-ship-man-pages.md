# ADR-0016: Ship man pages generated from the parser grammar

## Context and Problem Statement

The wrapper's documentation is reachable only through `--help` and the repository. A user who installs the binary has no `man claude-session`, which is the first place a Unix user looks. Whatever provides it must not become a second, hand-maintained description of the grammar.

## Considered Options

- No man pages — `--help` only.
- Hand-written roff or Markdown, converted at release time.
- Generate from the `clap` grammar, emitted by a `man` verb.

## Decision Outcome

Chosen option: generate from the grammar, exposed as a `man` verb.

The generator reads the same `Command` tree that produces `--help` and shell completions, so the flag list has exactly one source and cannot drift. The verb writes roff to standard output, or to a named directory, which lets a packager render pages at build time and lets a user preview one without installing anything.

Authored prose the parser cannot generate — the passthrough explanation, worked examples — comes from the same included text file that feeds the parser's long help, so it reaches both surfaces without being written twice.

Man pages describe the wrapper's grammar only, for the reason [ADR-0002](./ADR-0002-verbatim-argv-passthrough.md) gives: the child's flag list is not the wrapper's to track, and a stale copy of it would be worse than none.

## Consequences

- Good: a third documentation surface with no third source of truth.
- Good: packagers get roff without the project shipping pre-rendered files.
- Bad: one more claimed verb name, and one more dependency reading `clap`'s tree.
- Bad: generated roff is only partly controllable, so the page's shape follows the generator's opinions rather than ours.

## Status

Accepted

Amends [ADR-0003](./ADR-0003-reserve-a-small-wrapper-cli-surface.md) — the claimed verb set grows by one.
