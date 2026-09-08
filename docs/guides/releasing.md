# Release and publishing

This is the sole release and publishing runbook. Exact values and invariants live in [the release workflow reference](../reference/release-workflow.md).

The release convention is release-kit's ([ADR-0118](../decisions/ADR-0118-adopt-the-release-kit-trunk-convention.md)), so the ordered commands live there rather than here. `rk guide setup` carries the one-time repository and registry setup, `rk guide release` carries a release from landing the work to verifying the published version, and `rk method operate` explains each step. This page carries what is this project's own.

## Prerequisites

- Repository administrator access.
- `rk` on `PATH`. The devshell supplies it, pinned in [`flake.nix`](../../flake.nix).
- A green `just hooks`.
- [ADR-0022](../decisions/ADR-0022-cut-the-first-release-when-passthrough-works.md)'s passthrough criteria are satisfied before the first publish.

## Bootstrap release automation once

Every step in this section runs once, ever. A routine release repeats none of them.

1. Validate metadata and package contents with `./scripts/publish-dry`.
2. Run the repository-side setup by `rk guide setup`, whose steps 1 to 3 cover the trunk, the protections, and the auto-merge switch, and whose steps 5 to 8 cover the bot identity and the registry.
3. Prove what was applied with `rk setup check --target .`.
4. When ADR-0022 is satisfied, perform [the first manual publish](#first-manual-publish).
5. Register the crates.io Trusted Publisher against `release-plz.yml`, using the identity in the reference.
6. Revoke the bootstrap token.
7. Exercise an OIDC release and verify its source package, tag, and binary assets.
8. Enable crates.io "require trusted publishing" after the successful OIDC publish.

Ordering is load-bearing: the App bypass actor exists before any ruleset does. `rk guide setup` performs these in that order and says why at each step.

## First manual publish

This procedure ships `0.1.0` only after ADR-0022 is satisfied. Trusted Publishing can only be configured against a crate that already exists, so this upload is part of shipping `0.1.0` rather than a step taken before it.

1. Run `./scripts/publish-dry`.
2. Create the shortest-expiry crates.io token with endpoint `publish-new` and exact crate scope `claude-session`.
3. Run `cargo login` and enter that token.
4. Run `cargo publish --dry-run` again.
5. Run `./scripts/publish`.
6. Immediately return to Trusted Publisher setup and token revocation in the bootstrap procedure.

## Routine automated release

Every version after `0.1.0` is routine, and `rk guide release` is its command list. The slice that ships a rung carries that version's own documentation and gates — the claims it adds to `README.md`, the reference pages its surface changes, and any package content it introduces — as tail work inside the slice. Slice 009 is one-time first-release readiness and is never reopened for a later version.

The shape of it: work reaches `master` through a squash-merged pull request from its own worktree branch, release-plz opens or updates the release request, and that request merges itself once every required check is green. A release is held by disarming the request before its last check goes green. Never hand-create a tag and never push `master`.

## Prepare a release locally

These commands prepare or simulate a release; none publishes:

```bash
./scripts/release release-plz-update
./scripts/release release-plz-pr
./scripts/release cargo-release-dry <level>
```

`semver-check` is outside this binary-only release lane.

## Publish if CI is unavailable

1. Run `./scripts/publish-dry`.
2. Temporarily disable crates.io "require trusted publishing" if it is enabled.
3. Create a shortest-expiry token restricted to endpoint `publish-update` and exact crate `claude-session`.
4. Run `cargo login`, then `./scripts/publish`.
5. Revoke the token immediately.
6. Restore "require trusted publishing".

Local publishing uses a crates.io token; it does not use OIDC.

## Recover from a bad release

Releasing a new version is the recovery. Yanking is containment and fixes nothing on its own — it de-indexes the version so no new resolution picks it up, and every existing lockfile keeps resolving to it. See [version and recovery policy](../reference/release-workflow.md#version-and-recovery-policy), and `rk method recovery` for the convention's own recovery paths.

1. Yank the affected version with `cargo yank --version X.Y.Z`, to stop new consumers reaching it.
2. Fix the defect on a branch off `master`, through the ordinary request path.
3. Release a new compatible version through the routine automated path. This is the step that fixes it for anyone already on the bad version.
4. If the yank was mistaken, undo it with `cargo yank --version X.Y.Z --undo`.

If the bad release leaked a secret, rotate that secret now. A yank does not remove the published package and does not un-leak anything in it.

## Verify

1. Confirm `cargo package --list` contains only the intended source package.
2. Confirm the `vX.Y.Z` tag exists and crates.io serves that version.
3. Confirm the GitHub Release contains the single Linux archive, its checksum, the shell installer, and `dist-manifest.json` — [the published set](../reference/release-workflow.md#what-a-release-publishes), and nothing beyond it.
4. Confirm every file on that release carries a GitHub Artifact Attestation.
5. Confirm the tag and the cargo-dist run use the GitHub App-authored chain.
6. Confirm Trusted Publishing stays enabled and `rk setup check --target .` reports every step satisfied.
