# Foundation R1: Crate Manifest & Canonical Module Tree

> Plan: cs-foundation | Round: 1 of 4 | Complexity: L | Executor: prex (EF 1.5) | Generated: 2026-06-19 | Repo: repository root

## Context

`claude-session` is a Rust CLI wrapping the `claude` command. The hard contracts are native passthrough, XDG compliance, and self-containment. The repository is already bootstrapped: `Cargo.toml` contains package metadata, edition 2024, `rust-version = "1.97"`, and an empty `[dependencies]`; `Cargo.lock` exists; `rust-toolchain.toml` pins 1.97.1 with `rustfmt` and `clippy`; and `src/main.rs` is still the hello-world stub. There is no canonical `src/` module tree yet.

This round establishes only the absent manifest lint, binary, and release-profile sections plus the canonical shipped-crate module tree. It does not implement module behaviour, add speculative dependencies, create the later configuration generator, or change the queue graph.

## Previous Rounds

This is the first round; there are no prior rounds.

## Durable inputs

- [Implementation-round boundaries](../../../docs/reference/coding-conventions.md#implementation-round-boundaries) apply in full.
- [Architecture § Module roles](../../../docs/explanation/architecture.md#module-roles) owns the shipped module tree and dependency direction.
- [Architecture § One shipped crate, plus `xtask`](../../../docs/explanation/architecture.md#one-shipped-crate-plus-xtask) owns the tooling seam and library-surface limit.
- [ADR-0007](../../../docs/decisions/ADR-0007-layered-single-crate-architecture.md) owns the layer boundaries; [ADR-0014](../../../docs/decisions/ADR-0014-xtask-workspace-for-dev-tooling.md) records that the `xtask` trigger has fired.
- [Dependencies § Adding a dependency](../../../docs/reference/dependencies.md#adding-a-dependency) owns admission, graph selection, verification, and removal.

## Scope of this round

In scope:

- Add the absent `[[bin]]`, `[profile.release]`, `[lints.rust]`, and `[lints.clippy]` sections to `Cargo.toml` without changing existing package metadata.
- Add only a reviewed dependency that code in this round actually uses, through the admission procedure. An empty module tree requires none, so `[dependencies]` remains empty unless the implementation introduces a concrete in-scope use.
- Replace the hello-world stub with the canonical module declarations and create the canonical module files with meaningful module documentation and the minimum compiling content.

Out of scope:

- Real parsing, error variants, logging, configuration, output, services, adapters, and process spawning.
- The `xtask` generator, its development dependencies, and generated artifacts, which remain in the configuration-composition plan.
- Broad public re-exports or any public API not required by the documented tooling seam.
- Toolchain or package-metadata bootstrap work already present.

## Implementation steps

### First step: mark this round as started

In this plan's `queue-rounds.yaml`, set only `crate-manifest-and-module-tree` from `todo` to `doing`.

### Step 1: complete the manifest shape

Preserve the existing package metadata, edition, MSRV, and empty dependency table. Add the absent binary target, release profile, and lint tables using the exact values owned by [coding conventions](../../../docs/reference/coding-conventions.md#lints). Do not hand-edit a dependency entry or version. If a concrete in-scope need appears, stop and follow [the dependency procedure](../../../docs/reference/dependencies.md#adding-a-dependency); never pre-add a deferred crate.

### Step 2: create the canonical shipped module tree

Create the modules owned by [architecture](../../../docs/explanation/architecture.md#module-roles): `cli`, `commands`, `domain`, `services`, `adapters`, `config`, `context`, `error`, `logging`, `ui`, and `util`. Use post-2018 `foo.rs` plus sibling `foo/` form where children exist. Each module starts with documentation stating its role and prohibition; placeholders must be meaningful enough to satisfy lints without inventing behaviour.

Respect dependency direction and placement even in placeholders: parse shape stays in `cli`, domain stays pure, and no I/O appears outside adapters. Do not create `xtask` imports or a broad library re-export. ADR-0014 is current authority, but the later generator round owns creating and consuming the tooling seam.

### Step 3: replace the hello-world entry point

Replace the printing stub with module declarations and the minimum compiling `main`. Do not add output, parsing, environment reads, global state, or process exits. Later rounds own runtime wiring.

### Final step: verify and update the queue

Run the acceptance commands and the repository gate. Read the results against the boundaries below; file existence or compile success alone is insufficient. Only after every criterion passes, set this round's status to `done` in `queue-rounds.yaml`.

## Acceptance criteria

- `cargo metadata --locked`, `cargo check`, and `cargo clippy --all-targets --all-features -- -D warnings` succeed, followed by `nix develop --command pre-commit run --all-files`.
- `Cargo.toml` retains its existing package metadata and has the required binary, release-profile, and lint sections. No dependency version was hand-written and no deferred or unused dependency was added.
- Every canonical module exists with module documentation and follows [the implementation-round boundaries](../../../docs/reference/coding-conventions.md#implementation-round-boundaries).
- Boundary review rejects backwards imports, I/O outside adapters, mutable or ambient global state, terminal output outside its owner, and any OS-string conversion that could weaken passthrough.
- Tooling-isolation review rejects an `xtask` import in shipped code, a development-tooling crate in the shipped graph, or public surface beyond the documented tooling seam.
- No production `unsafe`, `panic!`, unjustified `unwrap` or `expect`, catch-all error, boxed return error, or direct `std::process::exit` is introduced.
- `src/main.rs` contains only the minimal entry-point shape and no hello-world output.
- The round's queue status changes only after all checks pass; no plan status or dependency edge changes.

## Next round

Round 2 (`errors-logging-context-config`) fills the typed error, logging, output, context, and configuration foundations under their durable owners.
