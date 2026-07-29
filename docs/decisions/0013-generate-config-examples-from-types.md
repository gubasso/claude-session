# ADR-0013: Generate config examples from types

## Context and Problem Statement

[The CLI surface](../reference/cli-surface.md) once listed an `init` verb that would "create the wrapper's configuration scaffold". That contradicts [ADR-0006](./0006-place-files-by-xdg-ownership.md): configuration is user-authored, and the wrapper only ever reads it. Users still need something accurate to start from, and a hand-written example drifts the first time a field is added, renamed, or made optional.

## Considered Options

- **Scaffold on `init`** — the wrapper writes a starter file into the config directory.
- **Hand-maintained examples** — authored files, kept correct by review.
- **Generate examples and a schema from the config types**, with a gate proving freshness.

## Decision Outcome

Chosen option: **generate from the config types** — copy-don't-scaffold. The type is the single source of truth and the example is derived from it, so it cannot drift.

A generator emits a JSON Schema and an annotated example per reflectable surface: required keys active, optional keys commented out, obviously fake placeholders, and a header naming the copy destination and stating that the wrapper never writes configuration. Generation **fails** when a public field carries no description, which is what keeps the example self-documenting rather than a wall of bare keys. A surface that cannot be reflected from a type — a settings piece is the child's own format — ships as a hand-maintained example under the same discipline.

Freshness is **byte comparison against freshly rendered text**, not a cache. Rendering is deterministic, so a stale file is exactly one whose contents differ, and an unrelated commit is a natural no-op. Nothing needs invalidating.

The artifacts, the header, and the gate are specified in [configuration](../reference/configuration.md).

## Consequences

- Good: examples cannot drift, and the schema doubles as editor and CI validation.
- Good: configuration stays read-only to the wrapper, so a Nix- or Home-Manager-managed file is never written to.
- Bad: adds a generator and a freshness gate to maintain.
- Bad: generated files must be staged whole, so partially staging one is a mistake the gate cannot catch.

## Status

Accepted

Replaces the configuration scaffold that [the CLI surface](../reference/cli-surface.md) formerly assigned to `init` — a specification line, not a prior decision record. The verb itself is retired by [ADR-0015](./0015-retire-the-init-verb.md).
