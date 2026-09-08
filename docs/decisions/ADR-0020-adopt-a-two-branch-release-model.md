# ADR-0020: Adopt a two-branch release model

## Context and Problem Statement

Releases are cut by automation, and what that automation publishes has to be reachable from something a packager, a bisecting user, or a downstream consumer can trust. The question is how many long-lived branches carry that job, and which of them a human may write to.

## Considered Options

- Trunk-only: one long-lived branch, releases cut by tagging it.
- GitFlow: long-lived `develop` and `master`, plus per-release and hotfix branches.
- Two long-lived branches with a one-way promote: `develop` integrates, and continuous integration fast-forwards `master` after a successful release.

## Decision Outcome

Chosen option: two long-lived branches with a one-way promote. Trunk-only leaves no branch that means "the published version", so answering "what is released right now" requires reading tags. GitFlow's release and hotfix branches solve a parallel-maintenance problem this project does not have.

`develop` is the integration branch and the release trigger. `master` mirrors the latest published version and is written only by the automated fast-forward. Feature branches are short-lived, kept linear by rebasing onto `develop`, and merged through a reviewed pull request. The mechanics are in [the release workflow](../reference/release-workflow.md).

## Consequences

- Good: `master` answers "what is published" without reading tags, and being machine-written it cannot drift from the answer.
- Good: the release trigger is a branch push, which is reviewable, rather than a hand-pushed tag.
- Bad: two branches to keep honest, and a contributor who targets the wrong one meets a confusing rejection.
- Bad: the invariant is only real under forge branch protection. Until that is configured, convention is the only thing stopping a human from writing to `master`.
- Bad: a push made with the default continuous-integration token does not retrigger workflows, so any future tag-driven job needs its own credential.

## Status

Superseded

Superseded by [ADR-0118](./ADR-0118-adopt-the-release-kit-trunk-convention.md), which replaces the two-branch model with one trunk named `master`. The outstanding external work this record named — the remote default branch, the GitHub App, and the rulesets — was never done and is now owned by the release-kit setup steps.
