# Account authentication

## Goal

Select, authenticate, inspect, and remove named accounts without taking ownership of child credentials.

## Appetite

4 implementation sessions.

## Core

Login and token modes are deterministic, private, redacted, and safe under concurrent rotation.

## In scope

- Directory discovery and last-used selection.
- Native login plus token ingest and transactional rotation.
- Launch precedence warnings and the scoped version floor.
- Account commands, verb-level JSON, redaction, and account health probes.

## Out of scope

- Reading, copying, refreshing, synchronizing, or fingerprinting child-owned credentials.
- Quota-aware failover or ambient authentication removal.

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
- [Testing and quality](../../../reference/testing-and-quality.md)

## Acceptance

- When accounts are listed, the wrapper shall derive them from directories without a registry index.
- When login mode succeeds, the child shall own one shared saved login under the account configuration directory and the wrapper shall not inspect it.
- When token mode ingests or rotates a token, the wrapper shall accept it only from a terminal or standard input and shall commit it under the credential lock without exposing it.
- If ambient authentication outranks a selected account, then the wrapper shall preserve it and warn without changing the stored mode.
- If login mode uses a below-floor or unparsable child, then the wrapper shall refuse before spawn while leaving token mode and unselected passthrough unblocked.
- When any account report is rendered, the wrapper shall redact credential material in every format and stream.

## Rabbit holes

- Wrapper-managed OAuth; escape: delegate native login and probe only through the child.
- TUI-login guarantees; escape: Q-004 must exit by measurement before tightening acceptance.

## Done when

Targeted account store, login, rotation, precedence, command, redaction, and health-probe tests pass under `cargo nextest`.

## Revisions

None.
