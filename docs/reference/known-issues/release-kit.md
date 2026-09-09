# release-kit

release-kit owns this project's release convention and the files its landing wrote ([ADR-0118](../../decisions/ADR-0118-adopt-the-release-kit-trunk-convention.md)). The adoption in September 2026 produced three real cases. Each one is filed upstream and worked around here.

## The dist profile is missing, so a first release ships no binaries

Upstream: [gubasso/release-kit#113](https://github.com/gubasso/release-kit/issues/113).

The landed workflow builds release artifacts with the `dist` Cargo profile. `dist generate` writes the workflow but never that profile, and only `dist init` writes it. A landed target therefore carries a workflow that names a profile its own `Cargo.toml` does not define.

Case: version `0.1.1` published its source to crates.io, then every artifact job failed on `profile 'dist' is not defined`. The source release is permanent, so `0.1.1` has binaries nowhere and no GitHub release page. Version `0.1.2` fixed it forward.

Workaround: `Cargo.toml` carries a `[profile.dist]` block with the reason beside it.

Remove when: release-kit's rust binding lands the profile or refuses a landing without one. The block stays either way if the binding only warns.

## A workspace root tags outside the protected pattern

Upstream: [gubasso/release-kit#114](https://github.com/gubasso/release-kit/issues/114).

The `release-tags` ruleset that `rk setup` installs protects `refs/tags/v*`. For a workspace root, release-plz defaults to `<crate>-v{{ version }}`, which that pattern does not match, so the release tag is deletable and movable by anyone with write access.

Case: this manifest is a workspace root with an `xtask` member, so the tag `claude-session-v0.1.1` exists outside the protection. It keeps its name, because renaming a published tag breaks anything that already resolved it.

Workaround: `release-plz.toml` pins `git_tag_name = "v{{ version }}"`.

Remove when: release-kit seeds that key for a workspace target, or its protection covers the workspace tag shape.

## The generated changelog fights the formatter

Upstream: [gubasso/release-kit#115](https://github.com/gubasso/release-kit/issues/115).

release-plz writes the changelog with its own default header and body template. This repository runs `dprint` over every Markdown file and forbids decorative emphasis. The default header wraps a sentence that `dprint` unwraps, and the default body writes a scope as italics and the breaking marker as bold.

Case: the emphasis contract failed on every release request, so no release could merge. After that was fixed, the trunk was dirty after each release, because the next `just hooks` run unwrapped the header again.

Workaround: `release-plz.toml` carries a `[changelog]` section with a rewrapped `header` and two `postprocessors`.

Remove when: release-kit either excludes the generated changelog from a target's formatter and prose gates, or seeds a template that survives them.
