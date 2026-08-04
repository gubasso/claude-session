# Release and publishing

This is the sole release and publishing runbook. Exact values and invariants live in [the release workflow reference](../reference/release-workflow.md).

## Prerequisites

- Repository administrator access.
- `develop` pushed and intended as the GitHub default branch.
- Conventional Commits on changes entering `develop`.
- A green `just hooks`.
- The release-plz, cargo-dist, workflow, and helper artifacts named in the reference are present.
- [ADR-0022](../decisions/ADR-0022-cut-the-first-release-when-passthrough-works.md)'s passthrough criteria are satisfied before the first publish.

## Bootstrap release automation once

Every step in this section runs **once, ever**. A routine release repeats none of them: it never touches forge settings, never creates a crates.io token, never registers a publisher, and never creates `master` — it fast-forwards a `master` that already exists. Step 5's ordering is the one that cannot be rearranged; see [forge enforcement](../reference/release-workflow.md#forge-enforcement).

1. Validate metadata and package contents with `./scripts/publish-dry`.
2. Push `develop` and make it the GitHub default branch.
3. Enable Actions read/write workflow permissions and “Allow GitHub Actions to create and approve pull requests”.
4. Register or reuse the account-wide GitHub App, grant Contents and Pull requests write, install it on this repository, and store `RELEASE_PLZ_APP_ID` and `RELEASE_PLZ_APP_PRIVATE_KEY`.
5. Create rulesets in lock-safe order: add the App bypass first, then protect `develop`, `master`, and `v*`.
6. When ADR-0022 is satisfied, perform [the first manual publish](#first-manual-publish).
7. Register the crates.io Trusted Publisher against `release-plz.yml` using the identity in the reference.
8. Revoke the bootstrap token.
9. Exercise an OIDC release and verify its source package, tag, `master` promotion, and binary assets.
10. Enable crates.io “require trusted publishing” after the successful OIDC publish.

## First manual publish

This procedure ships `0.1.0` only after ADR-0022 is satisfied. It does not authorize publishing the placeholder.

1. Run `./scripts/publish-dry`.
2. Create the shortest-expiry crates.io token with endpoint `publish-new` and exact crate scope `claude-session`.
3. Run `cargo login` and enter that token.
4. Run `cargo publish --dry-run` again.
5. Run `./scripts/publish`.
6. Immediately return to Trusted Publisher setup and token revocation in the bootstrap procedure.

## Routine automated release

1. Merge Conventional-Commit work through a reviewed pull request to `develop`.
2. Wait for release-plz to open or update its release pull request.
3. Review and merge that pull request; this is the sole human release decision.
4. Verify `vX.Y.Z`, the crates.io version, inline `master` promotion, and cargo-dist assets.
5. Never hand-create the tag or push `master`.

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
2. Temporarily disable crates.io “require trusted publishing” if it is enabled.
3. Create a shortest-expiry token restricted to endpoint `publish-update` and exact crate `claude-session`.
4. Run `cargo login`, then `./scripts/publish`.
5. Revoke the token immediately.
6. Restore “require trusted publishing”.

Local publishing uses a crates.io token; it does not use OIDC.

## Recover from a bad release

**Releasing a new version is the recovery. Yanking is containment and fixes nothing on its own** — it de-indexes the version so no new resolution picks it up, and every existing lockfile keeps resolving to it. See [version and recovery policy](../reference/release-workflow.md#version-and-recovery-policy).

1. Yank the affected version with `cargo yank --version X.Y.Z`, to stop new consumers reaching it.
2. Fix the defect on `develop`.
3. Release a new compatible version through the routine automated path. This is the step that fixes it for anyone already on the bad version.
4. If the yank was mistaken, undo it with `cargo yank --version X.Y.Z --undo`.

If the bad release leaked a secret, rotate that secret now. A yank does not remove the published package and does not un-leak anything in it.

## Verify

1. Confirm `cargo package --list` contains only the intended source package.
2. Confirm the `vX.Y.Z` tag exists and crates.io serves that version.
3. Confirm `master` resolves to the release tag's commit.
4. Confirm the GitHub Release contains the single Linux archive, its checksum, the shell installer, and `dist-manifest.json` — [the published set](../reference/release-workflow.md#what-a-release-publishes), and nothing beyond it.
5. Confirm the tag, cargo-dist run, and promotion use the GitHub App-authored chain.
6. Confirm Trusted Publishing and forge ruleset enforcement remain enabled.
