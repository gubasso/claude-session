# ADR-0127: Parse YAML with serde-saphyr

## Context and Problem Statement

Profiles are YAML. `serde_yaml_ng` parsed them through `unsafe-libyaml`, which [RUSTSEC-2024-0449](https://rustsec.org/advisories/RUSTSEC-2024-0449.html) marks unmaintained with no fixed version, so `cargo deny` failed on `master` until `deny.toml` ignored the advisory. The ignore was a stopgap: the parser itself had to go.

## Considered Options

- `yaml_serde`: the same API as `serde_yaml_ng`, parsing with `libyaml-rs`, which is libyaml translated to unsafe Rust by c2rust and has had no commit since March 2026.
- `serde-saphyr`: a serde layer over `granit-parser`, both written in safe Rust, with no value tree. The advisory names it for serde users.
- `serde_norway` and `serde_yml`: a fork of the same unsafe parser, and a crate with its own advisory, RUSTSEC-2025-0068.

## Decision Outcome

Chosen option: `serde-saphyr` with only its `deserialize` feature, because it removes unsafe parser code from the tree and every profile field is already typed. `src/domain/yaml.rs` sets each option that can change a parse result by name: YAML 1.1 booleans off, duplicate keys refused, `<<` an ordinary key, a `!!binary` string kept as text, application tags ignored, no legacy octal, non-finite floats passed through, the default budget, and no source snippet. A parse error reaches the user through its user formatter, which gives no advice about the parser's API. The other options are in the [ruled-out list](../reference/dependencies.md#ruled-out) and in `[bans].deny`.

## Consequences

- Good: `cargo deny` passes with no ignore, and no YAML parser in the tree uses unsafe code.
- Bad: one person maintains both crates, and they release often. The unmaintained check stays on, and the pinning tests in `src/domain/profile.rs` and `src/domain/strategy.rs` fail when an upgrade moves a parse result.
- Bad: some inputs read differently than before. An unquoted `null` layer, a core tag of another type, and 100 aliases of one anchor are refused, a null strategy table is empty, and a few YAML 1.2 spellings are accepted. [Configuration](../reference/configuration.md#how-a-profile-is-read) lists each.

## Status

Implemented

Enacted by [`src/domain/yaml.rs`](../../src/domain/yaml.rs), `Profile::parse` in [`src/domain/profile.rs`](../../src/domain/profile.rs), and `[bans].deny` in [`deny.toml`](../../deny.toml).
