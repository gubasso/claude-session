# ADR-0073: Defer OpenSSF Scorecard until the first release

## Context and Problem Statement

Scorecard is the ecosystem's default supply-chain self-assessment, and adopting it is a standing suggestion for any published Rust crate. This repository has no tagged release, one maintainer, and a binary that prints a placeholder. The question is whether to wire it in now or record why not.

## Considered Options

- Adopt it now, on a schedule against the default branch.
- Defer it, with a named trigger.
- Decline it permanently.

## Decision Outcome

Chosen option: **defer it, with a named trigger** — its central checks cannot pass a repository that has never released, so a published score would measure the absence of a release rather than the project's supply chain.

`Maintained`, `Signed-Releases`, `Code-Review`, `Contributors`, and `CI-Tests` all read release and collaboration history this repository does not have yet. Its Branch-Protection check additionally wants a maintainer token, which is the long-lived credential [ADR-0039](./ADR-0039-use-a-github-app-for-release-automation.md) deliberately replaced with a short-lived App token.

**Trigger:** after the first tagged release, once the `develop`, `master`, and `v*` rulesets are live and there is release history to read.

Declining permanently is rejected. The present coverage — `cargo audit`, `cargo deny`, two secret scanners, pinned actions, a pinned toolchain, and Trusted Publishing — is what Scorecard would measure, so the gap is evidence rather than practice, and evidence becomes available at the trigger.

## Consequences

- Good: no workflow, no `security-events` or `id-token` grant, and no published score that misleads about a placeholder binary.
- Good: the reason is recorded, so adoption is not re-argued before the trigger fires.
- Bad: no third-party attestation until the first release, which a downstream consumer with a policy may ask for sooner.

## Status

Accepted

Recorded as a not-adopted row in [release workflow § Forge enforcement](../reference/release-workflow.md#forge-enforcement).
