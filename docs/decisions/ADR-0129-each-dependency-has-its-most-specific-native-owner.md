# ADR-0129: Each dependency has its most specific native owner

## Context and Problem Statement

[ADR-0040](./ADR-0040-provision-hook-binaries-from-the-devshell.md) took `typos` from the devShell, because its wheel holds a binary that names the standard ELF loader, which a Nix host lacked. The operator's projects then took one rule on 2026-11-09: each dependency has one owner, the most specific file that declares it natively. The host now supplies the standard loader: a conventional distribution carries it, and the NixOS hosts get it from nix-ld.

## Considered Options

- Keep `typos` as a devShell tool, as ADR-0040 decided.
- Give each hook tool that pre-commit can build to `.pre-commit-config.yaml`, and keep a system hook only with a recorded owner and class.

## Decision Outcome

Chosen option: give each such tool to `.pre-commit-config.yaml`, because the reason for the system hook is gone and one tool has one owner.

- `typos` runs the upstream `crate-ci/typos` hook at the version the devShell carried, and `deadnix` the upstream `astro/deadnix` hook. Both leave the devShell, with `cargo-machete`, whose upstream hook already ran.
- Every hook that keeps `language: system` carries an `# owner:` comment with its class: `no-hook-language`, `project-environment`, `recorded-failure`, `outside-caller`, or `base-tool`.
- `markdownlint-cli2` keeps `language_version: system` on the devShell nodejs, as ADR-0040 decided.

## Consequences

- Good: a hook tool and its version sit beside the hook, and a run outside the devShell still gets them.
- Bad: a managed hook downloads its environment once on each machine.
- Bad: a NixOS host runs the `typos` wheel only after nix-ld is deployed there.

## Status

Implemented

Amends [ADR-0040](./ADR-0040-provision-hook-binaries-from-the-devshell.md) for `typos`. Enacted by [`flake.nix`](../../flake.nix) and [`.pre-commit-config.yaml`](../../.pre-commit-config.yaml).
