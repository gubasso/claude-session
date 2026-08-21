# 018 — Legacy coexistence namespace

## Goal

Let the wrapper and the shell predecessor it replaces be installed at the same time, so the predecessor stays available while the wrapper is proven, and neither reads, writes, or strips anything the other owns.

## Appetite

2 implementation sessions.

## Core

No name the wrapper reads or writes collides with a name the predecessor claims.

## In scope

- One decision recording the rename as deliberately temporary, naming the predecessor as the whole reason and the condition that ends it, so the return is a commitment rather than a remembered intention.
- The four XDG namespace directories, which are one call site under [ADR-0006](../../../decisions/ADR-0006-place-files-by-xdg-ownership.md)'s placement rule.
- The log filename, which shares the state base with the predecessor's own entries.
- The project configuration filename that [ADR-0070](../../../decisions/ADR-0070-discover-the-project-configuration-file-at-the-repository-root.md) discovers at a repository root.
- The command name, which is what the completion script registers and the man page titles, so those two installed artifacts stop colliding and stop misnaming the binary.
- The environment prefix and the child-environment scrub of [ADR-0057](../../../decisions/ADR-0057-build-the-child-environment-by-prefix-scrub-and-marker.md). The predecessor configures itself through `CLAUDE_SESSION_*`, so a scrub on that prefix deletes another program's configuration on the way to the child; a narrower prefix scrubs only what this wrapper set.
- Alignment of [XDG storage](../../../reference/xdg-storage.md), [configuration](../../../reference/configuration.md), and [CLI surface](../../../reference/cli-surface.md), and of the generated examples that print a destination path.

## Out of scope

- The cargo package and library name, which name a build unit rather than a filesystem or environment identity, and which no second install can collide with.
- The composed-entry domain tag, which separates hash domains rather than naming a path or a variable.
- Migrating either program's existing state, since coexistence means neither reads the other's directory.
- The return to the original namespace, which is 019 and waits on a condition this slice cannot meet.
- The friction entries the usability pass raised against help output for wrapper verbs.

## Governed by

- [AGENTS.md](../../../../AGENTS.md)
- [Charter](../../charter.md)
- [ADR-0006](../../../decisions/ADR-0006-place-files-by-xdg-ownership.md)
- [ADR-0051](../../../decisions/ADR-0051-let-every-surface-element-discriminate.md)
- [ADR-0057](../../../decisions/ADR-0057-build-the-child-environment-by-prefix-scrub-and-marker.md)
- [ADR-0070](../../../decisions/ADR-0070-discover-the-project-configuration-file-at-the-repository-root.md)
- [XDG storage](../../../reference/xdg-storage.md)
- [Configuration](../../../reference/configuration.md)
- [CLI surface](../../../reference/cli-surface.md)
- [Presentation](../../../reference/presentation.md)
- [Coding conventions](../../../reference/coding-conventions.md)
- [Testing and quality](../../../reference/testing-and-quality.md)
- [Development workflow](../../../guides/development-workflow.md)

## Acceptance

- When the wrapper resolves a path under any XDG base, the namespace directory shall carry the installed binary name. -> domain::paths::tests::every_managed_path_matches_the_artifact_table
- When the wrapper emits help, a man page, or a completion script, the command name shall be the installed binary name. -> cli_artifacts::generated_artifacts_name_the_installed_binary

## Rabbit holes

- Renaming the cargo package alongside the paths, which churns every build entry point and buys no separation, because two installs cannot collide in a manifest; escape: identity on disk and in the environment is the need.
- Writing a compatibility layer that reads the old namespace when the new one is empty, which recreates the coupling the slice exists to remove; escape: the predecessor keeps its directory and the wrapper starts clean.
- Treating the command-name change as the usability friction it also happens to fix; escape: this slice moves the name because two installed artifacts collide, and the help-output friction stays where it was raised.

## Done when

No path, filename, or environment variable the wrapper reads or writes carries a name the predecessor claims, the decision records the condition that ends the rename, 019 is shaped against that condition, and `just hooks` is green.

## Revisions

2026-08-21: the acceptance line asserting a scrub narrowed to this wrapper's own prefix, and the one requiring documents to name the coexistence spelling, were superseded by [019](../019-original-namespace-restoration/README.md) and removed with the test that proved the first. What was learned is that the narrowed prefix was the whole coexistence in miniature: it could only be correct while a second program owned the shorter one, so it had no meaning left the moment that program was gone.
