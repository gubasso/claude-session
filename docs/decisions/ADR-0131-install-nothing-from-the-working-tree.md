# ADR-0131: Install nothing from the working tree

## Context and Problem Statement

[ADR-0116](./ADR-0116-install-the-wrapper-one-way.md) made `just install`, through `install.sh`, the one way the wrapper reached `PATH`, and [ADR-0119](./ADR-0119-build-the-wrapper-from-the-flake.md) kept that route beside the flake package. The operator's hosts now install the wrapper from the Nix package, through the configuration repository that owns those hosts. A recipe that also installs into a `PATH` directory puts a second binary on the same host, and the two answer `claude-session` in turn as `PATH` changes. The operator ruled on 2026-11-12 that a project whose tool a host configuration installs keeps no recipe that installs it.

## Considered Options

- Keep `just install` beside the host install.
- Remove the recipes and the install scripts, and run a debug build from `target/`.

## Decision Outcome

Chosen option: remove the recipes and the scripts, because one host has one install of the wrapper.

- `install`, `uninstall`, and `reinstall` leave the `justfile`, and `install.sh`, `uninstall.sh`, and `install-common.sh` leave the repository.
- `just build` and `just run` are the development route. They build into `target/` and install nothing.
- `nix/package.nix` installs the bash completion with `installShellCompletion`, so a host install carries the completion with the binary. The expression is seeded by release-kit and owned by this project, so this record edits it.
- crates.io stays the install route for other users, as the README states.

## Consequences

- Good: a host carries one wrapper, and its completion comes from the same build.
- Good: three shell scripts and their maintenance leave the repository.
- Bad: a developer without the host configuration installs with `cargo install --path .`, which no recipe wraps.

## Status

Implemented

Supersedes [ADR-0116](./ADR-0116-install-the-wrapper-one-way.md). Amends [ADR-0119](./ADR-0119-build-the-wrapper-from-the-flake.md): `just install` no longer reaches `PATH`, and the package expression is edited here. Enacted by [`justfile`](../../justfile), [`nix/package.nix`](../../nix/package.nix), and [`flake.nix`](../../flake.nix).
