# ADR-0074: Enforce boundary rules with clippy configuration

## Context and Problem Statement

Four architectural rules are structural rather than type-checked, and the reference declares all four enforced by grep. No grep hook exists, so the mechanism is a promise. Two of the four also describe a Rust API being banned, which grep can only approximate: it matches text, so it fires inside a `///` example and misses an aliased import.

## Considered Options

- Write the four greps as pre-commit hooks, as the reference already claims.
- Configure clippy's `disallowed-*` lints for the two rules that ban an API, and keep grep for the two that do not.
- Leave all four unenforced and mark them deferred.

## Decision Outcome

Chosen option: clippy for the two API bans, grep for the two structural rules — a lint that resolves paths enforces an API ban exactly, and a text scan cannot.

Output ownership becomes `disallowed-macros` on the print macros; environment typing becomes `disallowed-methods` on `std::env::var` and `std::env::vars`, carrying `std::env::var_os` as the replacement. Both gain a `reason` that names this record. The wins are concrete: the reference currently has to warn that the output-ownership grep must not fire inside a `///` block, and the environment grep matches the literal `std::env::var(` while `use std::env; env::var(…)` is the common spelling.

Dependency direction and tooling isolation stay grep. They are module-graph and manifest facts, and clippy has no lint for either. The manifest half of tooling isolation is additionally reachable through `cargo-deny`'s ban list.

## Consequences

- Good: two rules move from a review promise to `clippy -D warnings`, which already gates every commit.
- Bad: enforcement is split across two mechanisms, so a reader checking a rule has to look in `clippy.toml` or in a hook depending on which rule it is.
- Bad: the two remaining greps still need writing, and until they exist the reference marks them deferred rather than enforced.

## Status

Accepted

The rules and their rejecting mechanisms stay owned by [testing and quality § Boundary lints](../reference/testing-and-quality.md#boundary-lints) and the hook configuration.
