# 005 — Account authentication

## Goal

Let a user launch `claude-session --account=work` in native login mode and discover the accounts available to select.

## Appetite

2 implementation sessions.

## Core

A selected login-mode account gives the child one private, durable configuration directory while unselected passthrough remains native.

## In scope

- Account-directory discovery without a registry and `account login` in native login mode.
- `CLAUDE_CONFIG_DIR` launch injection and a last-used account marker written before the exec.
- `account list` in human and verb-level JSON forms.
- Account-discovery and login-mode entries in the doctor catalog, with their feature-owned probes and remediation.
- User-facing documentation and per-rung release gates needed to make the account MVP ready for `0.2.0`.

## Out of scope

- Token ingest, rotation, status, removal, redaction, precedence warnings, and the scoped child version floor; slice 013 owns them.
- Reading, copying, refreshing, synchronizing, or fingerprinting child-owned credentials.
- Profile selection or composed settings.

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

- When accounts are listed, the wrapper shall derive them from directories without a registry index.
- When native login succeeds, the child shall own one shared saved login under the account configuration directory and the wrapper shall not inspect it.
- When a login-mode account is selected for launch, the wrapper shall set `CLAUDE_CONFIG_DIR`, set no wrapper token variable, and record the selection before the exec.
- When no account is selected, the wrapper shall inject neither wrapper authentication variable and shall preserve native passthrough.
- When the account MVP lands, its doctor catalog entries, user documentation, and `0.2.0` release gates shall describe only behavior this rung implements.

## Rabbit holes

- Token support while the account directory is being established; escape: keep every wrapper-owned secret and lifecycle command in slice 013.
- Wrapper-managed OAuth; escape: delegate native login to the child and never inspect its credential.

## Done when

Targeted account discovery, native login, launch environment, marker, listing, doctor-catalog, and passthrough tests pass under `cargo nextest`, the `0.2.0` rung documentation is honest, and `just hooks` is green.

## Revisions

None.
