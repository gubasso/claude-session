# ADR-0029: Use a credential-helper process boundary

## Context and Problem Statement

Long-lived subscription tokens need a persistent default store and an optional secure-store seam. Linking platform keyring backends would expand the dependency and platform matrix, while the headless-Linux keyutils backend documented by the Rust `keyring` family is memory-only and does not survive reboot.

## Considered Options

- Use a private file by default and an explicit argv-based helper process optionally.
- Link a keyring or secret-service library.
- Implement an in-process backend matrix.
- Silently fall back between helper and file storage.

## Decision Outcome

Chosen option: **an out-of-process argv-based helper seam** — the private file remains the default, and helper use is explicit.

Neither direction silently falls back to the other. The helper protocol and configuration schema are deferred to later work; this record chooses only the architectural boundary.

## Consequences

- Good: users can integrate platform or third-party secret stores without a linked backend matrix.
- Good: a configured helper failure cannot silently move a secret into a file, and a file failure cannot silently invoke an external program.
- Bad: helper configuration and protocol need a separate specification before implementation.
- Bad: the default store is a local private file rather than a system keyring.

See [ADR-0009](./ADR-0009-blessed-dependency-set.md), [dependencies](../reference/dependencies.md), and the public [`keyring::keyutils` documentation](https://docs.rs/keyring/latest/keyring/keyutils/).

## Status

Accepted
