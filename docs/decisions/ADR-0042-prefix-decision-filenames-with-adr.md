# ADR-0042: Prefix decision record filenames with `ADR-`

## Context and Problem Statement

Records were named `NNNN-short-title.md` while every reference to one — in prose, in a commit message, in a round file — calls it `ADR-NNNN`. The filename and the identity did not match, so a bare `0012-docs-architecture.md` in an editor tab, a search result, or a pasted path did not say what kind of document it was.

## Considered Options

- Keep the bare four-digit prefix and rely on the containing directory for identity.
- Prefix the filename with `ADR-`, matching the ID used everywhere else.
- Keep the filename and add an explicit ID line inside each record.

## Decision Outcome

Chosen option: **prefix the filename with `ADR-`**, giving `ADR-NNNN-short-title.md`. A record now carries its own identity wherever the path travels away from its directory, and `ADR-0012` finds the file, its heading, and every inbound link with one search.

`template.md` keeps its name: it is not a record and has no number.

## Consequences

- Good: filename, heading, and citation are the same string, so a record is greppable by its ID alone.
- Good: an open tab or a pasted path is unambiguous without its parent directory.
- Bad: a one-time rename of every record, rewriting every inbound link across the documentation, the round files, and the manifests that cite one.
- Bad: the prefix is redundant inside `docs/decisions/`, where nothing else is a record.

## Status

Accepted

Amends [ADR-0012](./ADR-0012-docs-architecture.md), which placed records in a zone but did not name them.
