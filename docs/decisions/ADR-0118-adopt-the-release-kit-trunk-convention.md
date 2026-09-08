# ADR-0118: Adopt the release-kit trunk convention

## Context and Problem Statement

[ADR-0020](./ADR-0020-adopt-a-two-branch-release-model.md) chose two long-lived branches, and the outstanding forge work it names was never done: no ruleset exists, no App is installed, and nothing has been published. Meanwhile release-kit owns this convention externally, as one method, one payload, and one set of executable setup steps. A private copy of a convention that has an owner duplicates a durable fact instead of carrying it.

## Considered Options

- Keep the two-branch model and finish its outstanding forge setup by hand.
- Adopt the release-kit convention, which names one trunk called `master`.
- Adopt the payload but keep `develop` as the branch name.

## Decision Outcome

Chosen option: adopt the release-kit convention. `master` becomes the sole permanent branch and the repository default, `develop` is renamed into it, and every change reaches the trunk through a short-lived branch that is squash-merged and deleted. The release style is `trunk`, so the bot's release request carries auto-merge from creation and a green trunk ships itself; the working-copy mode is `worktree`.

Keeping the branch name was rejected because release-kit does not make the trunk name configurable, so the choice is the whole convention or none of it. The landed payload, its record, and the setup steps live in `.release-kit/`, and the method is read with `rk method`.

## Consequences

- Good: the release convention has one owner outside this repository, and `rk status --check` and `rk setup check` judge the setup rather than a human reading prose.
- Good: one trunk, so no contributor can target the wrong branch.
- Bad: the rename is one-way, and every document naming `develop` had to be rewritten.
- Bad: `master` no longer means "what is published"; tags answer that, which is what ADR-0020 rejected.
- Bad: the project now depends on `rk`, supplied by the devshell pin in [`flake.nix`](../../flake.nix).

## Status

Accepted

Supersedes [ADR-0020](./ADR-0020-adopt-a-two-branch-release-model.md). Enacted by the landing record under `.release-kit/` and by the landed workflows.
