# ADR-0070: Discover the project configuration file at the repository root

## Context and Problem Statement

[Configuration](../reference/configuration.md#precedence) has always had a project layer "discovered by walking up from the working directory", with no filename and no rule for where the walk stops. A filename is a permanent contract and a stop rule decides whether a parent checkout, or `$HOME`, can silently configure a repository below it. Neither can be left to the implementation.

## Considered Options

- Walk to the filesystem root and unify every file found, as Cargo does.
- Walk until an explicit `root = true` marker key, as EditorConfig does.
- Walk to the enclosing repository root; first file found wins.
- No project layer at all — an environment variable naming one file, as ripgrep does.

## Decision Outcome

Chosen option: `.claude-session.toml`, first found wins, stopping at the enclosing repository root — the directory holding a `.git` entry, a file for worktrees and submodules and a directory otherwise. Git is never invoked. Outside a repository there is no project layer.

The stop rule follows the layer's stated purpose. These are per-repository overrides, so the repository is the boundary, and it is the only candidate needing no marker key to maintain, no ceiling variable to escape with, and no rule for merging several files. Nested repositories stop at the inner one, which is the answer a user working in a vendored checkout expects.

Unifying every ancestor was rejected because it makes a home-directory file a silent contributor to every repository. A marker key was rejected because it puts the stop rule in the data, where a repository that forgets it inherits its parent's configuration.

## Consequences

- Good: a project file's reach is exactly the repository containing it, checkable by eye.
- Bad: a working directory outside any repository has no project layer, so a non-repository tree cannot carry one.
- Bad: the walk stats for `.git` on every invocation, one syscall per level.

## Status

Accepted

Constrained by [ADR-0071](./ADR-0071-restrict-the-project-layer-to-the-profile-key.md), which decides what this file is permitted to set.
