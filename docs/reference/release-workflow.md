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

Promotion targets the release tag, never merely the workflow trigger SHA. The promotion job resolves the tag commit, requires it to be an ancestor of `origin/develop`, and uses `git merge --ff-only`. On the first release it may create `master` directly at the tag. [ADR-0020](../decisions/ADR-0020-adopt-a-two-branch-release-model.md) owns the branch decision.

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

[ADR-0037](../decisions/ADR-0037-publish-with-release-plz-and-trusted-publishing.md) owns registry publication; [ADR-0039](../decisions/ADR-0039-use-a-github-app-for-release-automation.md) owns the GitHub actor.

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

## What a release publishes

Every release produces exactly these, and nothing else:

| Artifact                                                                                                                         | Produced by                           |
| -------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------- |
| The crates.io source package, with the contents `Cargo.toml`'s `exclude` allows                                                  | `release-plz`, over OIDC              |
| The `vX.Y.Z` tag                                                                                                                 | `release-plz`, App-authored           |
| One GitHub Release, carrying one `x86_64-unknown-linux-gnu` archive, its checksum, the shell installer, and `dist-manifest.json` | `release.yml`, retriggered by the tag |
| The `master` fast-forward to the tag commit                                                                                      | the inline `promote` job              |
| The changelog entry                                                                                                              | `release-plz`, on the release PR      |

## Binary distribution

Source publication and binary distribution are orthogonal. Cargo-dist 0.32.0 builds one artifact, `x86_64-unknown-linux-gnu`, with a shell installer — the supported platform and nothing else ([ADR-0066](../decisions/ADR-0066-ship-one-linux-artifact.md)). No other architecture, no PowerShell installer, and no Homebrew tap.

The generated `release.yml` reads its build matrix from `dist plan` at run time and names no target, so changing `dist-workspace.toml` does not by itself require regenerating it. The App-authored release tag retriggers that workflow; its GitHub Release assets are consumable by cargo-binstall. [ADR-0038](../decisions/ADR-0038-distribute-binaries-with-cargo-dist.md) owns the choice of generator.

That retrigger constrains every tag-driven job added later: an App-authored tag push fires them, so a new job carries both a `needs:` on the release job and an `if:` on its outputs, as `promote` does. A standalone tag job runs on a tag it was never meant to see.

## Helper scripts

| Script                | Boundary                                                           |
| --------------------- | ------------------------------------------------------------------ |
| `scripts/publish-dry` | Token-free package dry run and package listing                     |
| `scripts/publish`     | Sole configuration-only authentication gate and real local publish |
| `scripts/release`     | Non-publishing preparation and dry-run dispatcher                  |

No helper validates or prints a token.

## Version and recovery policy

Semantic Versioning applies to the CLI surface. Removing or renaming a wrapper command or flag, or changing a default, is breaking; while the version is `0.x`, a breaking CLI change requires a minor bump. Native `claude` passthrough must never break.

**A version number is never reused, and that is the registry's rule rather than this project's.** A crates.io publish is permanent: the version cannot be overwritten and the code cannot be deleted.

**Recovery is a new version. Yank is containment, not recovery.** Yanking removes a version from the index so no new resolution picks it up; it deletes nothing, and every existing lockfile keeps working. So a yank stops the bleeding and changes nothing for anyone already affected — only a fix-forward release does that.

A yank also cannot un-publish a leaked secret. If one reached the package, rotate it immediately and treat the yank as irrelevant to the exposure.

## Forge enforcement

This is external state the repository cannot assert. Every row below is **required before the first release and currently unverified** — nothing in the gate reads the forge, so treat this as the target to apply, not a description of what is live.

| Required state                                                              | Operator action                                                     |
| --------------------------------------------------------------------------- | ------------------------------------------------------------------- |
| GitHub default branch is `develop`                                          | Set it in repository settings                                       |
| Actions have read/write permission and may create and approve pull requests | Enable both in Actions settings                                     |
| The GitHub App is installed and is `master`'s bypass actor                  | Register or reuse the App, install it, and add it as a bypass actor |
| `develop` is protected: reviewed, green pull requests                       | Create the `develop` ruleset                                        |
| `master` is App-only with linear history                                    | Create the `master` ruleset                                         |
| `v*` release tags are protected                                             | Create the tag ruleset                                              |

**Ordering is load-bearing: the App bypass actor exists before any ruleset does.** A ruleset created first locks the App out of the branch it is the only writer of, and recovering means an administrator relaxing the rule they just made. [The bootstrap procedure](../guides/releasing.md#bootstrap-release-automation-once) performs these in that order.

Rulesets are used rather than classic branch protection: they compose, they are readable by non-admins, they cover tags as well as branches, and they express a bypass actor explicitly — which is the whole mechanism `master` depends on.

The bypass actor must be the installed App. Naming `github-actions[bot]` instead fails with HTTP 422 from the ruleset API, because a personal account cannot use it as a bypass actor ([ADR-0039](../decisions/ADR-0039-use-a-github-app-for-release-automation.md)).

**Not adopted: OpenSSF Scorecard.** Deferred until after the first tagged release, with the trigger and the reasoning in [ADR-0073](../decisions/ADR-0073-defer-openssf-scorecard-until-the-first-release.md).

## Further reading

- [crates.io Trusted Publishing](https://crates.io/docs/trusted-publishing)
- [release-plz documentation](https://release-plz.dev/docs)
- [cargo-dist documentation](https://opensource.axo.dev/cargo-dist/)
- [GitHub App installation access tokens](https://docs.github.com/en/apps/creating-github-apps/authenticating-with-a-github-app/making-authenticated-api-requests-with-a-github-app-installation-access-token)
- [Semantic Versioning](https://semver.org/)
