# ADR-0119: Build the wrapper from the flake

## Context and Problem Statement

[ADR-0116](./ADR-0116-install-the-wrapper-one-way.md) removed `packages.default` because nothing consumed it and an unused build still has to be kept green. [ADR-0118](./ADR-0118-adopt-the-release-kit-trunk-convention.md) changes that reasoning: release-kit's Nix capability lands a package expression whose consumer is the gated pipeline, which proves the build on every request. A package with a consumer is no longer the unused surface ADR-0116 rejected.

## Considered Options

- Stay without a package, and let the release-kit landing withhold its Nix capability.
- Take the landed package expression and let the pipeline's `flake` job prove it.
- Take the expression and also publish a binary through Nix as a release channel.

## Decision Outcome

Chosen option: take the landed package expression. `nix/package.nix` is release-kit-owned and is never hand-edited; the flake exposes it, and the existing `flake` job in the gated pipeline is what compiles it on every pull request. `nix flake check` therefore becomes a real build check again, which is the incidental gate ADR-0116 noted it had lost.

Publishing through Nix was rejected on the discrimination test of [ADR-0051](./ADR-0051-let-every-surface-element-discriminate.md): the registry and the installers already answer distribution, and no current use separates a third channel from them.

`just install` stays the way the wrapper reaches `PATH` on a developer's host, so ADR-0116's single-install-path outcome survives; only its no-package outcome is replaced.

## Consequences

- Good: the build is proven by Nix on every request, from the same expression a consumer would use.
- Good: a consumer wanting `nix build` on this repository has it.
- Bad: a second build definition beside `Cargo.toml` to keep working, which is what ADR-0116 avoided.
- Bad: the expression is release-kit-owned, so changing it means changing the payload upstream.

## Status

Accepted

Amends [ADR-0116](./ADR-0116-install-the-wrapper-one-way.md), whose single-install-path outcome stands and whose no-package outcome this record replaces. Enacted by [`nix/package.nix`](../../nix/package.nix) and [`flake.nix`](../../flake.nix).
