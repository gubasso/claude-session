# release-kit

release-kit owns this project's release convention and the files its landing wrote ([ADR-0118](../../decisions/ADR-0118-adopt-the-release-kit-trunk-convention.md)). The adoption in September 2026 and the upgrade to 0.4.0 produced four real cases, and all four are closed. A generated changelog that fought the formatter closed when the checks stopped judging generated files, which the dev-environment rule of nix-secrets requires of every project. The other three closed upstream and keep a closing note below.

## Closed: the dist profile is missing

Upstream: [gubasso/release-kit#113](https://github.com/gubasso/release-kit/issues/113), closed.

Version `0.1.1` published its source to crates.io, then every artifact job failed on `profile 'dist' is not defined`, so `0.1.1` has binaries nowhere. release-kit now refuses a landing whose workflow names a `dist` profile that `Cargo.toml` does not define. The `[profile.dist]` block in `Cargo.toml` is the configuration that check asks for, and it stays.

## Closed: a workspace root tags outside the protected pattern

Upstream: [gubasso/release-kit#114](https://github.com/gubasso/release-kit/issues/114), closed.

This manifest is a workspace root with an `xtask` member, so release-plz defaulted to the tag `claude-session-v0.1.1`, which the `refs/tags/v*` ruleset does not protect. That tag keeps its name. release-kit now seeds `git_tag_name = "v{{ version }}"` in `release-plz.toml`, and this project keeps that line as normal configuration.

## Closed: the rendered security policy has no slot for the wrapper's scope

Upstream: [gubasso/release-kit#209](https://github.com/gubasso/release-kit/issues/209), closed.

release-kit 0.4.0 rendered `SECURITY.md` byte for byte, with no key for what a project does not own. Taking those bytes dropped this project's sentence that a vulnerability in the `claude` CLI or in the Anthropic API belongs to Anthropic, so the sentence moved to [the README](../../../README.md), where [ADR-0121](../../decisions/ADR-0121-report-a-vulnerability-through-the-forge.md) records it. A reporter who arrives through the forge's reporting form reads only `SECURITY.md`. release-kit 0.9.10 added `security.scope`, one paragraph that the policy renders after its reporting channel. `.release-kit/config.toml` sets it, so `SECURITY.md` carries the sentence again, and the README keeps it beside its pointer to the policy.
