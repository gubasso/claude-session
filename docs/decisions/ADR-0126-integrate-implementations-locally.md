# ADR-0126: Integrate implementations locally

## Context and Problem Statement

[ADR-0118](./ADR-0118-adopt-the-release-kit-trunk-convention.md) sent every change to `master` through a squash-merged pull request. One maintainer writes this project, so each request was a forge round trip that no reviewer read. release-kit records who moves an implementation onto the trunk as `git.integration`, and its default is `local`.

## Considered Options

- Keep forge integration: a pull request per change, merged by the forge behind the `gate` check.
- Integrate locally: `rk integrate` writes one squash commit on `master` from a worktree branch, and the operator pushes `master`.

## Decision Outcome

Chosen option: integrate locally, because a single writer gains nothing from a request that nobody reviews. `.release-kit/config.toml` records `integration = "local"` and `checkout_mode = "linked-worktree"`. Every code-changing branch still lives in its own worktree. `rk integrate` runs the `manual` hook stage, checks the message against the `commit-msg` judgment, and publishes the squash commit. The push is a separate fast-forward. `rk setup step protect-trunk` owns the trunk rulesets: deletion and force-push protection hold for every actor, and the repository-administrator role alone bypasses the request rules for the direct push.

The `manual` stage carries every commit-stage and push-stage hook, so the gate before the trunk is the whole suite. [Testing and quality](../reference/testing-and-quality.md#the-gate) owns that stage assignment. The release request still merges at the forge, so tags and publishing are unchanged.

## Consequences

- Good: no request per change, and several integrations can ride one push.
- Good: the whole local suite runs before a commit reaches the trunk, not only at a branch push.
- Bad: the remote checks start at the push, after the commit is on `master`. A red run blocks the release request through the `gate` check, and the repair is a new integration.
- Bad: continuous integration does not run `pre-commit`, so the hook-only checks guard the local side alone.
- Bad: a second writer must fetch, replay, and push again when the trunk moved.

## Status

Accepted

Amends [ADR-0118](./ADR-0118-adopt-the-release-kit-trunk-convention.md). Enacted by `git.integration` in `.release-kit/config.toml`, the `manual` stages in `.pre-commit-config.yaml`, and the `just hooks` recipe.
