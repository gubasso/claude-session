# Release workflow

This page owns exact release and publishing values and invariants. Operator tasks live in [the release guide](../guides/releasing.md).

## Branch and release invariant

| Branch                       | Role                                                           | Writer                    |
| ---------------------------- | -------------------------------------------------------------- | ------------------------- |
| `develop`                    | Integration branch, GitHub default branch, and release trigger | Reviewed pull requests    |
| `master`                     | Exact commit of the latest published release tag               | Installed GitHub App only |
| `feat/*`, `fix/*`, `chore/*` | Short-lived work based on `develop`                            | Contributors              |

```text
feature PR → develop → release PR → vX.Y.Z + crates.io → inline fast-forward to master
```

Promotion targets the release tag, never merely the workflow trigger SHA. The promotion job resolves the tag commit, requires it to be an ancestor of `origin/develop`, and uses `git merge --ff-only`. On the first release it may create `master` directly at the tag. [ADR-0020](../decisions/0020-adopt-a-two-branch-release-model.md) owns the branch decision.

## Source publication

Release-plz watches GitHub's default branch, `develop`; `release-plz.toml` has no branch key. The release pull request is the human release gate.

The `release-plz-release` job exposes the action outputs `releases_created` and `releases`. Its dependent inline `promote` job runs only when `releases_created == 'true'` and resolves the first release tag from the JSON `releases` output. Release-plz's default tag is `v{{ version }}`. Never create a release tag by hand.

## Authentication and automation actor

crates.io authentication and GitHub write identity are separate:

- crates.io uses Trusted Publishing. The release job has `id-token: write`; it has no `CARGO_REGISTRY_TOKEN` and no crates.io authentication action.
- GitHub writes use a short-lived installed-App token. Repository secrets are named `RELEASE_PLZ_APP_ID` and `RELEASE_PLZ_APP_PRIVATE_KEY`.

The crates.io Trusted Publisher identity is:

| Field             | Value             |
| ----------------- | ----------------- |
| Owner             | `gubasso`         |
| Repository        | `claude-session`  |
| Workflow filename | `release-plz.yml` |
| Environment       | blank             |
| Branch            | unrestricted      |

Never register `release.yml` as the publisher. The first publish uses a disposable, shortest-expiry token limited to endpoint `publish-new` and exact crate `claude-session`, then revokes it. Require Trusted Publishing only after one successful OIDC publish.

[ADR-0037](../decisions/0037-publish-with-release-plz-and-trusted-publishing.md) owns registry publication; [ADR-0039](../decisions/0039-use-a-github-app-for-release-automation.md) owns the GitHub actor.

## Workflow and configuration ownership

| File                                | Responsibility                                                          |
| ----------------------------------- | ----------------------------------------------------------------------- |
| `.github/workflows/ci.yml`          | Validation on `develop` pushes and all pull requests                    |
| `.github/workflows/release-plz.yml` | Release PR, source publish, tag creation, and inline `master` promotion |
| `.github/workflows/release.yml`     | Cargo-dist-generated tag workflow and GitHub Release assets             |
| `release-plz.toml`                  | Source-release policy                                                   |
| `dist-workspace.toml`               | Binary-distribution policy                                              |

`.github/workflows/release.yml` is generated and never hand-edited. Regenerate and check it with the pinned `dist`:

```bash
dist generate
dist generate --check
```

## release-plz policy

The live `[workspace]` policy is:

```toml
changelog_update = true
release_always = false
publish = true
semver_check = false
```

`semver_check` is disabled only because this crate is binary-only and has no Rust public API for cargo-semver-checks to compare. Its CLI compatibility is still versioned.

## Package metadata and contents

`Cargo.toml` owns all live package metadata and the anchored `exclude` denylist. Verify both buildability and contents:

```bash
cargo publish --dry-run
cargo package --list
```

The package includes Cargo files, Rust build inputs, `README.md`, both license files, and the changelog when one exists. It excludes project documentation, CI, helper scripts, release configuration, and development tooling.

An SPDX `license` expression does not make an `include` allowlist automatically carry a plain README or license files; an allowlist must name them. This crate therefore retains an `exclude` denylist. crates.io packages have a 10 MB ceiling.

## Binary distribution

Source publication and binary distribution are orthogonal. Cargo-dist 0.32.0 builds:

- `x86_64-unknown-linux-gnu`
- `aarch64-unknown-linux-gnu`
- `x86_64-apple-darwin`
- `aarch64-apple-darwin`

It generates shell and PowerShell installers. Windows targets and a Homebrew tap are not configured. The App-authored release tag retriggers generated `release.yml`; its GitHub Release assets are consumable by cargo-binstall. [ADR-0038](../decisions/0038-distribute-binaries-with-cargo-dist.md) owns this choice.

## Helper scripts

| Script                | Boundary                                                           |
| --------------------- | ------------------------------------------------------------------ |
| `scripts/publish-dry` | Token-free package dry run and package listing                     |
| `scripts/publish`     | Sole configuration-only authentication gate and real local publish |
| `scripts/release`     | Non-publishing preparation and dry-run dispatcher                  |

No helper validates or prints a token.

## Version and recovery policy

Semantic Versioning applies to the CLI surface. Removing or renaming a wrapper command or flag, or changing a default, is breaking; while the version is `0.x`, a breaking CLI change requires a minor bump. Native `claude` passthrough must never break.

Published versions are immutable. A yank prevents new dependency resolution but does not remove an artifact or break existing lockfiles. Recovery is yank if necessary, then fix forward with a new compatible version.

## Forge enforcement

The external repository state must enforce:

- GitHub default branch: `develop`.
- Actions: read/write workflow permission and permission to create and approve pull requests.
- GitHub App: installed before `master` protection and present as its bypass actor.
- `develop`: protected, reviewed, green pull requests.
- `master`: App-only writes and linear history.
- `v*`: protected release tags.

Apply these settings in [the bootstrap procedure](../guides/releasing.md#bootstrap-release-automation-once).

## Further reading

- [crates.io Trusted Publishing](https://crates.io/docs/trusted-publishing)
- [release-plz documentation](https://release-plz.dev/docs)
- [cargo-dist documentation](https://opensource.axo.dev/cargo-dist/)
- [GitHub App installation access tokens](https://docs.github.com/en/apps/creating-github-apps/authenticating-with-a-github-app/making-authenticated-api-requests-with-a-github-app-installation-access-token)
- [Semantic Versioning](https://semver.org/)
