# 013 — Token account lifecycle

## Goal

Extend named accounts with private token authentication, inspection, rotation, launch diagnostics, and removal.

## Appetite

2 implementation sessions.

## Core

Token state is ingested privately, committed transactionally, redacted everywhere, and never allowed to weaken native passthrough.

## In scope

- `account login --token` through a controlling terminal or standard input, candidate probing, and transactional rotation.
- `account status` and `account remove`, including confirmations, verb-level JSON, and safe failure reports.
- Launch precedence warnings and token-over-login reporting after Q-004 is measured.
- Credential redaction in every account report, diagnostic, format, and stream.
- Token-mode and lifecycle entries in the doctor catalog, with their feature-owned probes and remediation.
- Regenerated completion and man-page coverage plus user documentation and per-rung release gates for `0.3.0`.

## Out of scope

- Wrapper-managed OAuth, token refresh, upstream revocation, or ambient authentication removal.
- Reading, copying, refreshing, synchronizing, or fingerprinting child-owned credentials.
- Quota-aware failover, profiles, and composed settings.

## Governed by

- [AGENTS.md](../../../../AGENTS.md)
- [Session isolation](../../../explanation/session-isolation.md)
- [Accounts](../../../reference/accounts.md)
- [Process runtime](../../../reference/process-runtime.md)
- [XDG storage](../../../reference/xdg-storage.md)
- [Configuration](../../../reference/configuration.md)
- [CLI surface](../../../reference/cli-surface.md)
- [Exit codes](../../../reference/exit-codes.md)
- [Logging and output](../../../reference/logging-and-output.md)
- [Presentation](../../../reference/presentation.md)
- [Testing and quality](../../../reference/testing-and-quality.md)

## Acceptance

- When token mode ingests or rotates a token, the wrapper shall accept it only from a controlling terminal or standard input and shall commit it under the credential lock without exposing it.
- If ambient authentication outranks a selected account, then the wrapper shall preserve it and warn without changing the stored mode.
- Where token mode is stored, the wrapper shall leave the launch unblocked by the login-mode version floor slice 005 enforces.
- When account status is rendered, the wrapper shall report mode-aware health and metadata consistency without emitting credential material.
- When account removal is confirmed, the wrapper shall remove only the named local account state under the credential lock and shall state that upstream revocation did not occur.
- When any account report or failure is rendered, the wrapper shall redact credential material in every format and stream.
- When the token lifecycle lands, its doctor entries, generated CLI artifacts, user documentation, and `0.3.0` release gates shall agree with the implemented grammar.

## Rabbit holes

- Wrapper-managed OAuth; escape: delegate native login and probe tokens only through the child.
- TUI-login guarantees; escape: Q-004 must exit by measurement before warning and remediation behavior is finalized.

## Done when

Q-004 is resolved, targeted token ingest, rotation, precedence, status, removal, redaction, doctor-catalog, and generated-artifact tests pass under `cargo nextest`, the `0.3.0` rung documentation is honest, and `just hooks` is green.

## Revisions

None.
