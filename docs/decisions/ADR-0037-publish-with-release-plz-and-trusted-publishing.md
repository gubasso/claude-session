# ADR-0037: Publish with release-plz and Trusted Publishing

## Context and Problem Statement

Release-plz configuration and helper scripts exist, but the release tool and authentication choice lived only in root runbook prose. Registry releases need a reviewed release-PR gate without storing a long-lived registry secret.

## Considered Options

- Publish manually with `cargo publish`.
- Use release-plz with a long-lived `CARGO_REGISTRY_TOKEN`.
- Use release-plz with crates.io Trusted Publishing over OIDC.

## Decision Outcome

Chosen option: release-plz with crates.io Trusted Publishing over OIDC — release-plz opens or updates the release PR on the default `develop` branch, and merging it tags and publishes with `id-token: write` and no registry-token secret.

The first version remains the one-time manual exception required by [ADR-0022](./ADR-0022-cut-the-first-release-when-passthrough-works.md). See [the release workflow](../reference/release-workflow.md) for the contract and [the release guide](../guides/releasing.md) for procedures.

## Consequences

- Good: a reviewed merge is the human release decision and registry credentials are short-lived.
- Bad: the exact workflow filename is part of the Trusted Publisher identity.
- Bad: the first publish and Trusted Publisher registration remain manual external setup.

## Status

Accepted

This record formalizes an unrecorded tool and authentication choice. It neither supersedes nor amends another ADR and is consistent with ADR-0022 and [ADR-0023](./ADR-0023-only-release-automation-opens-pull-requests.md).

Amended by [ADR-0118](./ADR-0118-adopt-the-release-kit-trunk-convention.md): release-plz opens the release pull request against `master`, and `.github/workflows/release-plz.yml` is now release-kit-owned. The tool, the OIDC authentication, and the filename's place in the publisher identity are unchanged.
