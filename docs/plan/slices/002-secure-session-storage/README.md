# 002 — Secure session storage

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

- Per-key array strategies, structural validation, contributor provenance, child spawning, and authentication.
- Terminal-, project-, account-, or working-directory-derived entry keys.

## Governed by

- [AGENTS.md](../../../../AGENTS.md)
- [Session isolation](../../../explanation/session-isolation.md)
- [XDG storage](../../../reference/xdg-storage.md)
- [Configuration](../../../reference/configuration.md)
- [Exit codes](../../../reference/exit-codes.md)
- [Logging and output](../../../reference/logging-and-output.md)
- [Presentation](../../../reference/presentation.md)
- [Testing and quality](../../../reference/testing-and-quality.md)

## Acceptance

- If a storage component is a symlink or has the wrong owner, then the wrapper shall refuse it before reading or writing its leaf. -> session_storage::a_symlinked_managed_component_is_refused_before_its_leaf
- When a secure directory is used, the wrapper shall enforce mode `0700` on every invocation. -> session_storage::an_over_permissive_managed_directory_is_corrected_on_every_invocation
- When identical profile inputs are supplied, the wrapper shall name and reuse one immutable entry. -> session_storage::identical_inputs_name_and_reuse_one_immutable_entry
- If an existing entry disagrees with its full digest, then the wrapper shall refuse it without overwrite. -> session_storage::a_sidecar_digest_mismatch_is_refused_without_overwrite

## Rabbit holes

- Runtime-directory fallbacks; escape: durable state stays in the XDG state base or fails clearly.
- Entry cleanup and pruning; escape: immutable entries remain permanent for this slice.

## Done when

Targeted filesystem, identifier, store, and context tests pass under `cargo nextest`, followed by the repository hooks.

## Revisions

None.
