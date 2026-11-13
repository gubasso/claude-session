# ADR-0133: Release on the operator's order

## Context and Problem Statement

[ADR-0118](./ADR-0118-adopt-the-release-kit-trunk-convention.md) took the release-kit trunk style, in which the bot's release request carried auto-merge from creation, so a green trunk shipped itself. release-kit 0.9.15 retires that model: no landed job and no `rk` verb arms a release request or merges one without the operator's order. The same release moves the integration gate of [ADR-0126](./ADR-0126-integrate-implementations-locally.md) from the `manual` hook stage to a `pre-merge-commit` hook that git runs.

## Considered Options

- Stay on release-kit 0.9.10, which still arms the request.
- Take 0.9.15, and release only on the operator's order.

## Decision Outcome

Chosen option: take 0.9.15. A release request stays open and collects changes until the operator merges it with `rk release merge --target . --apply`, which dispatches the landed merge job and merges with the head pinned. A release is held by not giving that order. Staying on 0.9.10 keeps a model the convention's owner no longer maintains.

`rk integrate` runs the clone's `pre-merge-commit` hook on the squash. Every hook that runs at `manual` also runs at `pre-merge-commit`, so the gate before the trunk keeps the whole suite, and `just hooks` still reproduces it over every file.

## Consequences

- Good: no release ships without a person deciding it.
- Good: an ordinary nonbreaking `build` or `docs` commit does not itself provide release intent.
- Bad: a release now costs one operator command after the checks pass.
- Bad: a clone without the `pre-merge-commit` hook cannot integrate until `just hooks-install` installs it.

## Status

Implemented

Amends [ADR-0118](./ADR-0118-adopt-the-release-kit-trunk-convention.md) and [ADR-0023](./ADR-0023-only-release-automation-opens-pull-requests.md), whose release request no longer carries auto-merge, and [ADR-0126](./ADR-0126-integrate-implementations-locally.md), whose gate is now the `pre-merge-commit` hook. Enacted by [`release-plz.yml`](../../.github/workflows/release-plz.yml) and the `pre-merge-commit` stages in [`.pre-commit-config.yaml`](../../.pre-commit-config.yaml).
