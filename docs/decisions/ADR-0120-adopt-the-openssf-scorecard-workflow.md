# ADR-0120: Adopt the OpenSSF Scorecard workflow

## Context and Problem Statement

[ADR-0073](./ADR-0073-defer-openssf-scorecard-until-the-first-release.md) deferred Scorecard behind a named trigger: a first tagged release, live rulesets, and release history to read. All three now hold. Five versions are on crates.io, the `master-protection` and `release-tags` rulesets are installed, and every change since the adoption reached the trunk through a squash-merged request.

## Considered Options

- Adopt it, and publish the result to the public API.
- Adopt it, and keep the result in this repository's security tab.
- Keep deferring it, or decline it permanently.

## Decision Outcome

Chosen option: adopt it and publish, because the value is a supply-chain reading a consumer can check without trusting this repository's own prose.

`.github/workflows/scorecard.yml` runs on a push to `master` and weekly. It gates nothing. No job in it reports on a pull request, and the required check stays `gate` in `ci.yml`. Its shape follows the publisher's rules rather than this project's taste, because the API rejects a result from a workflow carrying top-level `env`, a workflow-level write permission, or `id-token: write` on a second job.

Two checks will score below their maximum, knowingly. Branch-Protection reads protection settings through a long-lived personal token, which [ADR-0039](./ADR-0039-use-a-github-app-for-release-automation.md) replaced with a short-lived App token. Pinned-Dependencies wants a commit SHA for every action, and this project pins a version tag so `rk versions --check` can read it. Both are decisions rather than gaps, and changing either is its own record.

## Consequences

- Good: a third party measures what this repository claims, weekly, and publishes the result.
- Good: a regression in the release path shows as a falling score rather than as silence.
- Bad: two checks score low for recorded reasons, so the number needs this record to read it.
- Bad: one more scheduled workflow, and four more action pins to keep current.

## Status

Implemented

Enacted in [the Scorecard workflow](../../.github/workflows/scorecard.yml). Supersedes [ADR-0073](./ADR-0073-defer-openssf-scorecard-until-the-first-release.md).
