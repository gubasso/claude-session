# ADR-0023: Only release automation opens pull requests

## Context and Problem Statement

The gate detects but never proposes: `cargo-audit` and `cargo-deny` report advisories and licence violations without offering an upgrade, and nothing inspects the GitHub Actions the workflows pin by mutable tag. The usual answer is a dependency-update bot. [ADR-0009](./ADR-0009-blessed-dependency-set.md) makes the set reviewed and deliberate, added only with `cargo add` and never by writing a version string into the manifest — which is exactly what such a bot does.

## Considered Options

- Adopt a dependency-update bot for crates and workflow actions both.
- Adopt one for workflow actions only, where ADR-0009 does not reach.
- Keep release automation as the only bot and update dependencies by hand.

## Decision Outcome

Chosen option: only release automation opens pull requests — `release-plz` opens the release pull request, and nothing else opens one. Machine-proposed bumps move the dependency set on a schedule the project did not choose, and a stream of generated pull requests is reviewed less carefully than the deliberate change ADR-0009 asks for.

An upgrade is therefore an ordinary human change: run `cargo update` or edit the pin, target `develop`, and let the gate decide whether it lands.

## Consequences

- Good: every change to the dependency set and to the pinned actions has an author who chose it.
- Good: no third-party forge service holds write access to the pull-request stream.
- Bad: pinned actions drift silently behind their tags, so their currency is a tracked perishable fact ([research-tracking](../reference/research-tracking.yaml)) rather than an automated one.
- Bad: an advisory the gate reports arrives with no fix waiting; someone has to write it.

## Status

Accepted
