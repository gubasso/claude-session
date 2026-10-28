# unsafe-libyaml

`serde_yaml_ng` 0.10 parses YAML through `unsafe-libyaml`, and this crate reads YAML through `serde_yaml_ng` in `src/domain/merge.rs`, `src/domain/profile.rs`, and `src/domain/strategy.rs`.

## The YAML parser is unmaintained and has no safe upgrade

Upstream: [RUSTSEC-2024-0449](https://rustsec.org/advisories/RUSTSEC-2024-0449).

The advisory marks `unsafe-libyaml` unmaintained. It names no fixed version, and it points to YAML parsers written in safe Rust, such as `saphyr` with `serde-saphyr`, or `noyalib`.

Case: in October 2026 `cargo deny check advisories` began to fail on `master` with no change in this repository. The `cargo-deny` hook runs in the `manual` stage, so `just hooks` and every `rk integrate` failed.

Workaround: `deny.toml` ignores the advisory with a reason. `scripts/report-ignored-advisories` prints each ignored advisory and its reason on every `just hooks`, every push, and every `just deny`, and as a warning annotation in CI, so the ignore never goes silent.

Remove when: `serde_yaml_ng` is replaced by a safe-Rust YAML crate. Then delete the `deny.toml` entry and this page, and drop the row from the known-issues index.
