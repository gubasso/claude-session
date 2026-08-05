# Secure session storage

## Goal

Expose secure XDG-backed account/profile paths and immutable composed-entry storage through diagnostics.

## Appetite

3 implementation sessions.

## Core

Every created component refuses symlinks and wrong ownership and enforces private modes.

## In scope

- Validated account/profile identifiers and deterministic input-digest entry keys.
- A write-once composed settings pair with mismatch refusal and partial-pair recovery.
- Lazy account/profile context and diagnostic paths.

## Out of scope

- Settings merge semantics, child spawning, and authentication.
- Terminal-, project-, account-, or working-directory-derived entry keys.

## Governed by

- [AGENTS.md](../../../../AGENTS.md)
- [Session isolation](../../../explanation/session-isolation.md)
- [XDG storage](../../../reference/xdg-storage.md)
- [Configuration](../../../reference/configuration.md)
- [Exit codes](../../../reference/exit-codes.md)
- [Logging and output](../../../reference/logging-and-output.md)
- [Testing and quality](../../../reference/testing-and-quality.md)

## Acceptance

- If a storage component is a symlink or has the wrong owner, then the wrapper shall refuse it before reading or writing its leaf.
- When a secure directory is used, the wrapper shall enforce mode `0700` on every invocation.
- When identical profile inputs are supplied, the wrapper shall name and reuse one immutable entry.
- If an existing entry disagrees with its full digest, then the wrapper shall refuse it without overwrite.

## Rabbit holes

- Runtime-directory fallbacks; escape: durable state stays in the XDG state base or fails clearly.
- Entry cleanup and pruning; escape: immutable entries remain permanent for this slice.

## Done when

Targeted filesystem, identifier, store, and context tests pass under `cargo nextest`, followed by the repository hooks.

## Revisions

None.
