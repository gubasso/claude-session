# 012 — Human presentation

## Goal

Make the presentation contract reach bytes, so the colour decision is visible and its absence is provable.

## Appetite

2 implementation sessions.

## Core

Each named surface carries colour when the ladder says so, and the plain reading of every surface is unchanged.

## In scope

- One colour decision resolved once and carried to every human surface, replacing the value discarded at the context constructor.
- The three named surfaces: the diagnostic kind, the mirrored level word, and the composed delimiter.
- A presentation crate admitted through the dependency procedure.
- One line of authored help prose naming the variable that turns colour off, which no flag can lead a user to.
- Two distinct precedence defects: empty `NO_COLOR` and `FORCE_COLOR` values must be inert, and active `FORCE_COLOR` must win over active `NO_COLOR`.
- Escape-byte assertions, including absence under `NO_COLOR`, a redirected stream, and JSON mode, plus forced colour on a redirected stream.

## Out of scope

- Tables, progress indicators, spinners, and interactive prompts.
- A wrapper flag for colour, which the passthrough contract forbids.
- The doctor report renderer, which slice 007 owns.
- Any new verb, document, or output surface.

## Governed by

- [AGENTS.md](../../../../AGENTS.md)
- [Presentation](../../../reference/presentation.md)
- [Logging and output](../../../reference/logging-and-output.md)
- [CLI surface](../../../reference/cli-surface.md)
- [Exit codes](../../../reference/exit-codes.md)
- [Coding conventions](../../../reference/coding-conventions.md)
- [Dependencies](../../../reference/dependencies.md)
- [Testing and quality](../../../reference/testing-and-quality.md)
- [ADR-0017](../../../decisions/ADR-0017-declare-a-human-facing-cli.md)
- [ADR-0081](../../../decisions/ADR-0081-bind-every-human-surface-to-one-presentation-contract.md)
- [ADR-0082](../../../decisions/ADR-0082-colour-a-closed-set-of-named-surfaces.md)
- [ADR-0083](../../../decisions/ADR-0083-read-only-the-two-published-colour-variables.md)

## Acceptance

- Where the target stream is a terminal and no override forbids it, the wrapper shall write each named surface with colour.
- When `NO_COLOR` is active and `FORCE_COLOR` is not active, the wrapper shall write no escape byte on any stream.
- When `NO_COLOR` and `FORCE_COLOR` are both active, the wrapper shall write colour on a human surface.
- When `FORCE_COLOR` is active for a redirected human stream, the wrapper shall write colour.
- While the output mode is JSON, the wrapper shall write no escape byte on either stream.
- The wrapper shall resolve the colour decision once and carry it to every renderer.
- When colour is off, the wrapper shall write the same text it writes when colour is on.
- The wrapper shall name the colour convention in its help output.
- Where `NO_COLOR` is present and empty, the wrapper shall treat it as unset.
- Where `FORCE_COLOR` is present and empty, the wrapper shall treat it as unset.

## Rabbit holes

- Building a palette or a theme; escape: colour the three named surfaces and stop.
- Adding a `--color` flag to settle a precedence argument; escape: the ladder is environment-only and the spelling belongs to the child.
- Colouring the child's inherited bytes; escape: the wrapper decorates only what it wrote.
- Reaching for a terminal-control crate to reuse its detection; escape: the ladder is four rungs over an environment snapshot.

## Done when

Each named surface carries an escape byte under a terminal, under active `FORCE_COLOR` on a redirected human stream, and when both colour variables are active; it carries none under unopposed `NO_COLOR`, an ordinary redirected stream, or JSON. The plain text is byte-identical either way, and `just hooks` is green.

## Revisions

None.
