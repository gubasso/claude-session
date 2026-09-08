# ADR-0040: Provision pre-commit tool binaries from the devShell

## Context and Problem Statement

Three hooks — `typos`, `committed`, and `markdownlint-cli2` — installed cleanly and then failed at exec with a bare `ENOENT`. Each obtains a prebuilt binary naming `/lib64/ld-linux-x86-64.so.2` as its ELF interpreter: crate-ci ships the first two as `language: python` wheels wrapping a generic-glibc executable, and pre-commit's `language: node` falls back to nodeenv, which downloads a generic-glibc node. That loader does not exist on a Nix host, so the breakage is permanent rather than flaky, and it reads as a content failure — a run that reports it as one records a false verdict.

## Considered Options

- Keep the upstream hooks and accept that three gates never run locally.
- Switch to the `-src` hook variants, compiling each from source in the hook environment.
- Take the binaries from the flake devShell and run the hooks as `language: system`.
- Wrap the foreign binaries with `patchelf` or `nix-ld`.

## Decision Outcome

Chosen option: take the binaries from the devShell — the project already promises the devShell provides the surrounding tools, and nixpkgs carries `typos` and `committed` at the exact versions this repository had pinned.

`typos` and `committed` become `repo: local` hooks with `language: system`, mirroring the upstream args so behaviour is unchanged. `markdownlint-cli2` stays an upstream hook pinned to `language_version: system`: pre-commit chooses system node on its own only when `node` and `npm` resolve outside `$HOME`, which a home-profile Nix install does not satisfy. npm still installs that hook's pure-JS custom rule, which nixpkgs does not package.

## Consequences

- Good: every gate runs, with no compile step, no download, and no foreign binary at hook time.
- Good: hook tool versions are pinned by `flake.lock` and bump as a reviewable commit.
- Bad: those versions now live in `flake.nix` rather than beside the hook, and a contributor working outside the devShell must supply the tools.

## Status

Implemented

Enacted by [`flake.nix`](../../flake.nix) and [`.pre-commit-config.yaml`](../../.pre-commit-config.yaml).

Amended by [ADR-0118](./ADR-0118-adopt-the-release-kit-trunk-convention.md): `committed` is gone, because the landed release-kit block owns commit-message linting and one job takes one hook. The provisioning rule this record states is unchanged and still governs `typos` and `markdownlint-cli2`.
