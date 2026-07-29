# ADR-0019: Pin the toolchain exactly and declare an MSRV

## Context and Problem Statement

`rust-toolchain.toml` named the `stable` channel and nothing else. That is a moving target: the compiler changes under the project without a commit, and a new clippy lint turns a green tree red for reasons no diff explains. The devShell claims to read the channel, components, and targets from that file, but the file declared no components, so `rustfmt` and `clippy` arrived by luck. The crate also promised no minimum supported compiler.

## Considered Options

- **Keep `stable`** and accept the drift.
- **Pin an exact version** and list the required components.
- **Pin, and additionally declare `rust-version`** as a compatibility promise.

## Decision Outcome

Chosen option: **pin exactly and declare `rust-version`**.

`rust-toolchain.toml` names one release and lists `rustfmt` and `clippy` explicitly, so every developer, the devShell, and CI compile with the same compiler and lint set — and the devShell's claim about what the file provides becomes true. Upgrading then arrives as a visible commit that can be reviewed, bisected, and reverted like any other change.

`rust-version` states the oldest compiler the crate supports, so `cargo` refuses an unsupported build with a clear message instead of failing deep inside a dependency.

The two are separate promises and may diverge: the pin is what this project builds with, the MSRV is what it supports. Raising the MSRV is a breaking change for consumers and belongs in release notes; lowering it is not.

## Consequences

- Good: reproducible builds, and a lint change arrives as a reviewable commit rather than a surprise.
- Good: `cargo` rejects an unsupported compiler early and legibly.
- Bad: someone must actually perform toolchain bumps, and a stale pin misses upstream fixes.
- Bad: the MSRV constrains which dependency versions can be adopted.

## Status

Accepted
