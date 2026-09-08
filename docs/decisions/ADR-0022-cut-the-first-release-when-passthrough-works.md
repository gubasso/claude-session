# ADR-0022: Cut the first release when passthrough works

## Context and Problem Statement

The crate builds a placeholder binary, but the release machinery is already wired: `release-plz.toml`, the helper scripts under `scripts/`, and [the release runbook](../guides/releasing.md) could publish today. crates.io versions are immutable — a published version can be yanked but never replaced — so the timing of the first upload is a one-way door. It also decides what `README.md` may promise and which guides can honestly be written.

## Considered Options

- Publish `0.1.0` from the current placeholder to reserve the name and exercise the pipeline; ship the working wrapper as `0.2.0`.
- Publish `0.0.1` as an obviously-empty bootstrap upload, keeping `0.1.0` for the first working release.
- Publish nothing until the passthrough contract works, and let that version be `0.1.0`.

## Decision Outcome

Chosen option: publish nothing until passthrough works. The crate's description promises forwarding to `claude`; a published version that does not forward is a permanently broken artifact for anyone who runs `cargo install claude-session`, and yanking only stops new dependents.

`0.1.0` is the first release. It is cut once the wrapper forwards argv, standard streams, and the child's exit status per [ADR-0002](./ADR-0002-verbatim-argv-passthrough.md) and [the exit-code taxonomy](../reference/exit-codes.md). Trusted Publishing attaches to a crate that already exists, so the one-time manual `publish-new` upload is part of shipping `0.1.0`, not a step taken before it.

## Consequences

- Good: every published version does what the crate description says, so nothing needs yanking.
- Good: one rule governs what user-facing documentation may promise — nothing until `0.1.0` exists. The `guides/` zone stays at its contributor guide rather than filling with tasks no reader can verify.
- Bad: the crate name is unreserved until then.
- Bad: release-plz, Trusted Publishing, and the `master` promote job get their first end-to-end exercise on the real `0.1.0` rather than on a throwaway version.

## Status

Accepted

`Cargo.toml` holds `0.1.0` as the unreleased authoring version; the tag that mirrors it does not exist yet.

Documentation timing now follows [ADR-0075](./ADR-0075-build-through-the-current-slice.md). The release decision recorded here is unchanged.

Amended by [ADR-0118](./ADR-0118-adopt-the-release-kit-trunk-convention.md): there is no `master` promote job any more, because `master` is the trunk. The first version and the one-time manual publish are unchanged.
