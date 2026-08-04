# ADR-0038: Distribute release binaries with cargo-dist

## Context and Problem Statement

crates.io distributes source, while this Unix CLI should also provide prebuilt artifacts that do not require a Rust toolchain. The former publishing runbook deferred binary distribution without recording a decision.

## Considered Options

- Require source installation with `cargo install`.
- Maintain custom release-build scripts.
- Generate the binary-distribution workflow with cargo-dist.

## Decision Outcome

Chosen option: **generate the binary-distribution workflow with cargo-dist** — cargo-dist 0.32.0 owns `dist-workspace.toml` and the generated `.github/workflows/release.yml`, targets the four supported Linux and macOS architectures, and stays separate from `release-plz.yml`.

## Consequences

- Good: releases provide archives and installers that cargo-binstall can discover.
- Good: the generator owns platform build and GitHub Release mechanics.
- Bad: generated YAML must be regenerated when its configuration changes.
- Bad: no Windows artifact or Homebrew tap exists until separately adopted.

## Status

Implemented

Implemented by [`dist-workspace.toml`](../../dist-workspace.toml) and the generated [release workflow](../../.github/workflows/release.yml). This status is scoped to binary-distribution configuration and does not assert implementation of the placeholder wrapper crate. This record neither supersedes nor amends another ADR; it replaces only an unrecorded deferral.

Amended by [ADR-0066](./ADR-0066-ship-one-linux-artifact.md) — the generator stands, the four-target and two-installer list does not.
