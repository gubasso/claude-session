# Release workflow

This page owns the release values this project decides for itself. The convention around them belongs to release-kit ([ADR-0118](../decisions/ADR-0118-adopt-the-release-kit-trunk-convention.md)): `rk method model` states the branch and merge model, `rk method invariants` states what must stay true, and `rk guide release` carries the operator's steps. Read the owner before changing a rule.

## Branch and release invariant

`master` is the only permanent branch and the repository default. Every change reaches it through a short-lived branch that is squash-merged and deleted, and every code-changing branch lives in its own worktree. Nothing is committed on `master` and no tag is authored by hand.

```text
worktree branch → pull request → squash to master → release PR → vX.Y.Z + crates.io + release assets
```

The release style is `trunk`: the bot's release request carries auto-merge from creation, so a green trunk ships itself. A release is held by disarming that request before its last check goes green.

## Authentication and automation actor

crates.io authentication and GitHub write identity are separate:

- crates.io uses Trusted Publishing. The release job has `id-token: write`; it has no `CARGO_REGISTRY_TOKEN` and no crates.io authentication action.
- GitHub writes use a short-lived installed-App token.

The crates.io Trusted Publisher identity is:

| Field             | Value             |
| ----------------- | ----------------- |
| Owner             | `gubasso`         |
| Repository        | `claude-session`  |
| Workflow filename | `release-plz.yml` |
| Environment       | blank             |
| Branch            | unrestricted      |

Never register `release.yml` as the publisher. The first publish uses a disposable, shortest-expiry token limited to endpoint `publish-new` and exact crate `claude-session`, then revokes it. Require Trusted Publishing only after one successful OIDC publish.

[ADR-0037](../decisions/ADR-0037-publish-with-release-plz-and-trusted-publishing.md) owns registry publication; [ADR-0039](../decisions/ADR-0039-use-a-github-app-for-release-automation.md) owns the GitHub actor.

## Workflow and configuration ownership

| File                                | Responsibility                                                             | Owner        |
| ----------------------------------- | -------------------------------------------------------------------------- | ------------ |
| `.github/workflows/ci.yml`          | Validation on `master` pushes and all pull requests, and the required gate | this project |
| `.github/workflows/pr-title.yml`    | The Conventional Commits check on the request title                        | release-kit  |
| `.github/workflows/release-plz.yml` | Release request, source publish, and tag creation                          | release-kit  |
| `.github/workflows/release.yml`     | Cargo-dist-generated tag workflow and GitHub Release assets                | cargo-dist   |
| `release-plz.toml`                  | Source-release policy                                                      | this project |
| `dist-workspace.toml`               | Binary-distribution policy                                                 | this project |
| `nix/package.nix`                   | The package expression the pipeline builds                                 | this project |

A release-kit-owned file is never hand-edited; `rk status --check` reports drift and the `rk-status-check` hook runs it. A file this project owns was seeded by the landing and is tuned here.

`.github/workflows/release.yml` is generated and never hand-edited. Regenerate and check it with the pinned `dist`:

```bash
dist generate --mode ci
dist generate --mode ci --check
```

## release-plz policy

The live `[workspace]` policy is:

```toml
changelog_update = true
release_always = false
publish = true
git_release_enable = false
semver_check = false
```

`semver_check` is disabled only because the `claude_session` library target exists for this crate's own tests and no external consumer holds that API. The CLI compatibility is still versioned. `git_release_enable` is false because cargo-dist creates the GitHub Release, being the half that has the installers to attach.

## Package metadata and contents

`Cargo.toml` owns all live package metadata and the anchored `exclude` denylist. Verify both buildability and contents:

```bash
cargo publish --dry-run
cargo package --list
```

The package includes Cargo files, Rust build inputs, `README.md`, the three license files, and the changelog when one exists. It excludes project documentation, CI, helper scripts, release configuration, and development tooling. A denylist entry anchors one path, so a directory of development tooling needs its own entry rather than inheriting the one that excludes a similarly named file.

An SPDX `license` expression does not make an `include` allowlist automatically carry a plain README or license files; an allowlist must name them. This crate therefore retains an `exclude` denylist. crates.io packages have a 10 MB ceiling.

## What a release publishes

Every release produces exactly these, and nothing else:

| Artifact                                                                                                                         | Produced by                           |
| -------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------- |
| The crates.io source package, with the contents `Cargo.toml`'s `exclude` allows                                                  | `release-plz`, over OIDC              |
| The `vX.Y.Z` tag                                                                                                                 | `release-plz`, App-authored           |
| One GitHub Release, carrying one `x86_64-unknown-linux-gnu` archive, its checksum, the shell installer, and `dist-manifest.json` | `release.yml`, retriggered by the tag |
| A GitHub Artifact Attestation over every file on that release                                                                    | `release.yml`, in its host phase      |
| The changelog entry                                                                                                              | `release-plz`, on the release request |

## Binary distribution

Source publication and binary distribution are orthogonal. Cargo-dist 0.32.0 builds one artifact, `x86_64-unknown-linux-gnu`, with a shell installer — the supported platform and nothing else ([ADR-0066](../decisions/ADR-0066-ship-one-linux-artifact.md)). No other architecture, no PowerShell installer, and no Homebrew tap.

The generated `release.yml` reads its build matrix from `dist plan` at run time and names no target, so changing the target list in `dist-workspace.toml` does not by itself require regenerating it. Changing the installer list does. The App-authored release tag retriggers that workflow; its GitHub Release assets are consumable by cargo-binstall. [ADR-0038](../decisions/ADR-0038-distribute-binaries-with-cargo-dist.md) owns the choice of generator.

`pr-run-mode` is `skip`, so this workflow reports nothing on a request. A forge resolves a job's `needs` inside one file, so the one required check lives in `ci.yml` and reaches only the jobs declared there.

That retrigger constrains every tag-driven job added later: an App-authored tag push fires them, so a new job carries both a `needs:` on the release job and an `if:` on its outputs. A standalone tag job runs on a tag it was never meant to see.

## Helper scripts

| Script                | Boundary                                                           |
| --------------------- | ------------------------------------------------------------------ |
| `scripts/publish-dry` | Token-free package dry run and package listing                     |
| `scripts/publish`     | Sole configuration-only authentication gate and real local publish |
| `scripts/release`     | Non-publishing preparation and dry-run dispatcher                  |

No helper validates or prints a token.

## Version and recovery policy

Semantic Versioning applies to the CLI surface. Removing or renaming a wrapper command or flag, or changing a default, is breaking; while the version is `0.x`, a breaking CLI change requires a minor bump. Native `claude` passthrough must never break.

A version number is never reused, and that is the registry's rule rather than this project's. A crates.io publish is permanent: the version cannot be overwritten and the code cannot be deleted.

Recovery is a new version. Yank is containment, not recovery. Yanking removes a version from the index so no new resolution picks it up; it deletes nothing, and every existing lockfile keeps working. So a yank stops the bleeding and changes nothing for anyone already affected — only a fix-forward release does that.

A yank also cannot un-publish a leaked secret. If one reached the package, rotate it immediately and treat the yank as irrelevant to the exposure.

## Forge enforcement

This is external state the repository cannot assert, and it is no longer tracked as a dated manual reading. `rk setup check --target .` proves every row against the forge, step by step, and `rk setup --target . --apply --required-check <name>` re-asserts them. [The release guide](../guides/releasing.md) carries the order and what stays the operator's.

Rulesets are used rather than classic branch protection: they compose, they are readable by non-admins, they cover tags as well as branches, and they express a bypass actor explicitly. The bypass actor must be the installed App; naming `github-actions[bot]` instead fails with HTTP 422 from the ruleset API, because a personal account cannot use it as a bypass actor ([ADR-0039](../decisions/ADR-0039-use-a-github-app-for-release-automation.md)).

Not adopted: OpenSSF Scorecard. Deferred until after the first tagged release, with the trigger and the reasoning in [ADR-0073](../decisions/ADR-0073-defer-openssf-scorecard-until-the-first-release.md).

## Further reading

- [crates.io Trusted Publishing](https://crates.io/docs/trusted-publishing)
- [release-plz documentation](https://release-plz.dev/docs)
- [cargo-dist documentation](https://opensource.axo.dev/cargo-dist/)
- [GitHub App installation access tokens](https://docs.github.com/en/apps/creating-github-apps/authenticating-with-a-github-app/making-authenticated-api-requests-with-a-github-app-installation-access-token)
- [Semantic Versioning](https://semver.org/)
