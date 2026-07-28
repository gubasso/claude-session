# Publishing

How this crate is published to [crates.io](https://crates.io). Helper scripts live under `scripts/` (`publish-dry`, `publish`, `release`). The auth checks in those scripts are **configuration checks only** — they confirm crates.io auth is set up, never that a token is valid.

## Decisions for `claude-session`

Recorded at bootstrap so this runbook is self-contained:

| Decision                  | Choice                                               | Why                                                                                                                                                                      |
| ------------------------- | ---------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Crate kind                | `bin` (CLI wrapping `claude`)                        | `src/main.rs`, no library target                                                                                                                                         |
| Auth mode                 | Trusted Publishing (OIDC) on GitHub Actions          | remote is GitHub; no long-lived `CARGO_REGISTRY_TOKEN`                                                                                                                   |
| First publish             | **manual**, with a scoped `publish-new` token        | Trusted Publishing is configured against an already-existing crate                                                                                                       |
| Release tool              | `release-plz`                                        | publishing happens from CI                                                                                                                                               |
| Release workflow filename | **`release-plz.yml`** (owned by the CI bootstrap)    | the crates.io Trusted Publisher matches on filename                                                                                                                      |
| SemVer gate               | `semver_check = false` in `release-plz.toml`         | bin-only crate has no public API to diff; the CLI contract is versioned by policy, not by `cargo-semver-checks`                                                          |
| Binary distribution       | cargo-dist **deferred**, not configured              | no prebuilt-binary release channel is in use yet, and `install.sh`/`uninstall.sh` already cover cargo-built local installs; see the cargo-dist section to adopt it later |
| License                   | `MIT OR Apache-2.0` (both `LICENSE-*` files present) | Rust dual-license convention                                                                                                                                             |

## Publishing model

- **CI-first (recommended):** `release-plz` opens a release PR that bumps the version and updates the changelog, `Cargo.toml`, and `Cargo.lock`. Merging that PR publishes the new version automatically.
- **Local (escape hatch):** run the helper scripts by hand when CI is unavailable.

First-time crates.io setup — creating a scoped `publish-new` token, `cargo login`, the first manual `cargo publish`, then configuring Trusted Publishing and revoking the token — is a **one-time manual requirement**, not part of routine maintenance. See the crates.io Trusted Publishing docs: <https://crates.io/docs/trusted-publishing>.

## SemVer policy

For a library crate, `cargo-semver-checks` gates public-API compatibility and runs natively inside release-plz. Run it locally with `./scripts/release semver-check`. A binary-only crate has no public API to check but still follows semantic versioning for its releases.

`claude-session` is binary-only, so `semver_check` is set to `false` in `release-plz.toml` and `./scripts/release semver-check` is not part of its release path. The versioned contract is the CLI surface instead: a removed/renamed subcommand or flag, or a changed default, is **breaking**. Under `0.x` that means a **minor** bump. Never break native `claude` passthrough in any release.

## Routine automated release

1. Merge feature work to the default branch.
2. release-plz opens/updates the release PR (version bump + changelog).
3. Review the PR; merge it.
4. release-plz tags the release and publishes to crates.io.

## Local operator release

When you need to drive a release by hand:

- `./scripts/release release-plz-update` — update versions + changelog locally.
- `./scripts/release release-plz-pr` — open/refresh the release PR.
- `./scripts/release cargo-release-dry <level>` — dry-run a `cargo-release` bump.
- `./scripts/release semver-check` — check API compatibility.

## Readiness checks

`./scripts/publish-dry` runs `cargo publish --dry-run` and `cargo package --list`. Neither needs a token; run it any time to confirm the package builds and ships the intended files.

## Package contents (keep the tarball lean)

Cargo packages the whole working tree by default, so project docs, CI, and dev tooling ship as dead weight unless trimmed. Check `cargo package --list` and keep the `.crate` to build inputs plus `README`/`LICENSE`/`CHANGELOG`. Prefer an `exclude` denylist in `Cargo.toml` — it is robust against dropping future `src/` files:

```toml
[package]
exclude = [
    "/docs",
    "/.github",
    "/scripts",
    "/release-plz.toml",
    "/dist-workspace.toml",
    "/justfile",
    "/flake.nix",
    "/.pre-commit-config.yaml",
]
```

Footgun: with an SPDX `license` expression (e.g. `MIT`), Cargo does **not** auto-include a plain `README` or `LICENSE`, so an `include` allowlist must list them explicitly. crates.io enforces a hard 10 MB limit; for a binary crate no consumer reads the tarball at all, so docs and tooling are pure waste.

This crate's live `exclude` list is already in `Cargo.toml`. Verified tarball contents at bootstrap: `Cargo.toml`, `Cargo.lock`, `src/main.rs`, `rust-toolchain.toml`, `README.md`, `LICENSE-MIT`, `LICENSE-APACHE`, `.gitignore` — 10 files, ~8 KiB compressed. Re-check with `./scripts/publish-dry` after adding any top-level file.

## Binary distribution (cargo-dist)

If this crate ships prebuilt binaries (a CLI/app), `dist` (cargo-dist) builds them and attaches shell/PowerShell/Homebrew-tap installers to each GitHub Release; `cargo-binstall` then works automatically from those releases. It is separate from crates.io publishing and configured in `dist-workspace.toml`. `dist` generates its own workflow at `.github/workflows/release.yml` — a **distinct file** from the release-plz workflow (`release-plz.yml`), so the two never collide and the crates.io Trusted Publisher keeps matching the actual release-plz filename. Treat the generated YAML as an artifact: run `dist init` (first time) or `dist generate` after editing `dist-workspace.toml`, never hand-edit it. AUR, OBS/zypper, and Homebrew (beyond the generated tap) are downstream/manual channels that consume the tagged GitHub Release artifacts — not auto-generated pipelines.

## Manual release if CI is down

1. `./scripts/publish-dry` to validate.
2. `./scripts/release semver-check` (library crates).
3. Ensure auth is configured (`cargo login`).
4. `./scripts/publish`.

## Yank and rollback

A published version cannot be overwritten or deleted, only yanked:

- `cargo yank --version X.Y.Z` — prevent new dependents from selecting it.
- `cargo yank --version X.Y.Z --undo` — reverse a yank.

Fix forward by publishing a new patch version. Under `0.x`, Cargo treats the **minor** as the breaking position (`0.y` bumps may break), so version accordingly.
