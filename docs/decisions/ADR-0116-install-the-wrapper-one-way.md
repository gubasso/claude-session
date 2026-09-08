# ADR-0116: Install the wrapper one way

## Context and Problem Statement

The flake exposed a `packages.default` that built the wrapper, while the devShell deliberately excluded it and a comment beside that exclusion said `just install` owns the binary on `PATH`. Two install paths existed and only one was used: nothing in this repository, in the operator's configuration repository, or in the container image consumed the flake package — the image builds the wrapper from its own source input, and the host runs `just install`. A build nobody runs still has to keep working, and a second way to install is a second answer to "which binary is this".

## Considered Options

- Expose no package; `just install` is the only install path.
- Keep `packages.default` as an unused but maintained build.
- Keep it and adopt it, retiring `just install`.

## Decision Outcome

Chosen option: expose no package — one install path, and the one already in use. `flake.nix` keeps the devShell and the formatter, which are what the repository actually depends on it for. The pinned-toolchain `rustPlatform`, the `Cargo.toml` read, and the explicit source file set existed only to feed the package and went with it.

Adopting the flake package instead was rejected because it makes installing the wrapper require Nix, which nothing else about the project does, and because the crate is published to a registry that `cargo install` already reaches.

Keeping it unused was rejected on the discrimination test of [ADR-0051](./ADR-0051-let-every-surface-element-discriminate.md): a surface with no current use is removed, and a rejected option is recorded rather than shipped early.

## Consequences

- Good: one binary, one way to get it, and no unused build to keep green.
- Bad: `nix flake check` no longer compiles anything. The compile gate is `just hooks`.
- Bad: a consumer wanting `nix build` on this repository must restore the output.

## Status

Implemented

Enacted by [`flake.nix`](../../flake.nix), whose devShell comment names this record, and by [`install.sh`](../../install.sh), which `just install` runs.

Amended by [ADR-0119](./ADR-0119-build-the-wrapper-from-the-flake.md), which restores a package output the pipeline consumes. The single install path stands.
