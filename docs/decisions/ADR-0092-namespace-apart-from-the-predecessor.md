# ADR-0092: Namespace apart from the predecessor until it is deprecated

## Context and Problem Statement

A shell implementation of this tool is still installed and still in use, and it claims `claude-session` under the config and state bases and the `CLAUDE_SESSION_` environment prefix. The wrapper claims the same names, so the two share a configuration tree, interleave entries in one state directory, and read each other's variables. The prefix is the sharpest edge: the child-environment scrub of [ADR-0057](./ADR-0057-build-the-child-environment-by-prefix-scrub-and-marker.md) removes the whole prefix, so the wrapper deletes the other program's configuration on the way to the child.

## Considered Options

- Namespace the wrapper as `claude-session-rs` until the predecessor is deprecated.
- Keep both on `claude-session` and separate them with per-shell XDG base overrides.
- Deprecate the predecessor now.

## Decision Outcome

Chosen option: namespace as `claude-session-rs` — the wrapper is the newcomer, so the cost of moving falls on it rather than on a working install.

Every name the wrapper reads or writes moves together: the four XDG namespace directories, the log filename, the project file of [ADR-0070](./ADR-0070-discover-the-project-configuration-file-at-the-repository-root.md), the command name that titles the man page and registers the completion, and the environment prefix with its scrub. Paths alone would leave the scrub deleting variables the wrapper never set.

Base overrides were rejected because they make correctness depend on which shell started the process. Deprecating now was rejected because it removes the fallback before the replacement is proven.

The rename is temporary and its end is scheduled work, not an intention: [019](../plan/slices/019-original-namespace-restoration/README.md) returns every name once the predecessor is deprecated, and supersedes this record when it lands.

## Consequences

- Good: both programs are installable at once, and neither can read, overwrite, or strip the other's state.
- Bad: the wrapper's on-disk identity differs from its project name until 019.
- Bad: state written before this change is stranded and re-created.

## Status

Implemented

Enacted by [`src/domain/paths.rs`](../../src/domain/paths.rs) and [`src/services/child.rs`](../../src/services/child.rs). Amends [ADR-0057](./ADR-0057-build-the-child-environment-by-prefix-scrub-and-marker.md) and [ADR-0070](./ADR-0070-discover-the-project-configuration-file-at-the-repository-root.md), which each name a literal this moves.
