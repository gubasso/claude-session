# release-kit

release-kit owns this project's release convention and the files its landing wrote ([ADR-0118](../../decisions/ADR-0118-adopt-the-release-kit-trunk-convention.md)). The adoption in September 2026 and the upgrade to 0.4.0 produced four real cases. One is open below, filed upstream and worked around here. Three are closed. A generated changelog that fought the formatter closed when the checks stopped judging generated files, which the dev-environment rule of nix-secrets requires of every project. The other two closed upstream and keep a closing note below.

## Closed: the dist profile is missing

Upstream: [gubasso/release-kit#113](https://github.com/gubasso/release-kit/issues/113), closed.

Version `0.1.1` published its source to crates.io, then every artifact job failed on `profile 'dist' is not defined`, so `0.1.1` has binaries nowhere. release-kit now refuses a landing whose workflow names a `dist` profile that `Cargo.toml` does not define. The `[profile.dist]` block in `Cargo.toml` is the configuration that check asks for, and it stays.

## Closed: a workspace root tags outside the protected pattern

Upstream: [gubasso/release-kit#114](https://github.com/gubasso/release-kit/issues/114), closed.

This manifest is a workspace root with an `xtask` member, so release-plz defaulted to the tag `claude-session-v0.1.1`, which the `refs/tags/v*` ruleset does not protect. That tag keeps its name. release-kit now seeds `git_tag_name = "v{{ version }}"` in `release-plz.toml`, and this project keeps that line as normal configuration.

## The rendered security policy has no slot for the wrapper's scope

Upstream: [gubasso/release-kit#209](https://github.com/gubasso/release-kit/issues/209).

release-kit 0.4.0 owns `SECURITY.md` as a rendered file, byte for byte. Its two tunable keys, `security.contact` and `security.response`, name who receives a report and how fast an answer arrives. Neither states what a project does not own, so a wrapper has nowhere to put its boundary.

Case: this project's own policy said that a vulnerability in the `claude` CLI, in the Anthropic API, or in a dependency belongs to whoever owns that code. Taking the rendered bytes at 0.4.0 dropped that sentence, and [ADR-0121](../../decisions/ADR-0121-report-a-vulnerability-through-the-forge.md) states as a consequence that the policy carries it. A reporter who arrives through the forge's own reporting form reads `SECURITY.md` and nothing else.

Workaround: the sentence lives in [the README](../../../README.md) beside the pointer to the policy, and ADR-0121 names both enactment points.

Remove when: release-kit renders a project-supplied scope paragraph into the policy, or gives the target a marked region inside that file.
