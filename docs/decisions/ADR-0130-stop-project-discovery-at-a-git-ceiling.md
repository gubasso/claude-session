# ADR-0130: Stop project discovery at a git ceiling

## Context and Problem Statement

[ADR-0070](./ADR-0070-discover-the-project-configuration-file-at-the-repository-root.md) stops the project-file walk at the first directory holding a `.git` entry, and it never invokes git. A `.git` entry above the working directory therefore always makes that directory part of a repository, even where git itself would not look. A test tree under `/tmp` read a project file once, because a repository stood above `/tmp` on the host, and no setting could tell the walk to stop below it.

## Considered Options

- Keep the walk as it is, and rerun a test that meets a host repository.
- Honour `GIT_CEILING_DIRECTORIES` with git's meaning.
- Add a ceiling variable of the wrapper's own.

## Decision Outcome

Chosen option: honour `GIT_CEILING_DIRECTORIES` with git's meaning, because a user who bounds git's discovery with it then gets the same repository from the wrapper as from git.

The walk never enters a listed directory from below, so a `.git` entry at or above a ceiling is outside the working directory's repository, and there is no project layer. A ceiling equal to the working directory does not stop the walk. Empty and relative entries are skipped. As in git, an entry before the first empty one is resolved through its symbolic links and dropped when it cannot be, and an entry after it is compared as written. Git is still never invoked. The test harness sets the variable to the parent of each fixture, so a host repository cannot reach a fixture tree.

A ceiling variable of the wrapper's own was rejected, because it would be a second spelling of a boundary that users already state to git.

## Consequences

- Good: a project file applies only where git would find the same repository, and the tests no longer depend on the host.
- Bad: the wrapper now reads one variable that git owns, and must follow git's reading of it.

## Status

Implemented

Enacted in `src/config/project.rs` and in [the configuration reference](../reference/configuration.md#project-file-discovery). Amends [ADR-0070](./ADR-0070-discover-the-project-configuration-file-at-the-repository-root.md).
