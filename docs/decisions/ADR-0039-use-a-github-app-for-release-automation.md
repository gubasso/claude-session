# ADR-0039: Use a GitHub App for release automation

## Context and Problem Statement

Pushes made with the default workflow token do not trigger tag workflows, and `github-actions[bot]` cannot be a personal-account ruleset bypass actor. Release-plz must trigger cargo-dist, while promotion must write protected `master`.

## Considered Options

- Use the default `GITHUB_TOKEN`.
- Store a personal access token.
- Use an SSH deploy key.
- Mint tokens for a least-privilege installed GitHub App.

## Decision Outcome

Chosen option: **mint tokens for a least-privilege installed GitHub App** — one account-wide App has repository Contents and Pull requests write permission, is installed per repository, and is the `master` ruleset bypass actor. Workflows mint short-lived tokens from `RELEASE_PLZ_APP_ID` and `RELEASE_PLZ_APP_PRIVATE_KEY`.

## Consequences

- Good: tag retriggering and protected-branch bypass use a non-person-bound identity.
- Good: each job narrows the token to the permissions it needs.
- Bad: App registration, installation, secrets, and bypass configuration are external setup that must follow the guide's lock-safe order.

## Status

Accepted

Workflow consumption is implemented, while forge setup remains manual. This record does not amend [ADR-0020](./ADR-0020-adopt-a-two-branch-release-model.md); it implements that record's credential consequence.
