# 005 — Account authentication

## Goal

Let a user launch `claude-session --account=work` in native login mode and discover the accounts available to select.

## Appetite

2 implementation sessions.

## Core

A selected login-mode account gives the child one private, durable configuration directory. Slice 017 later changed the unselected leg to a pre-exec refusal.

## In scope

- Account-directory discovery without a registry and `account login [name]` in native login mode.
- `CLAUDE_CONFIG_DIR` launch injection and a last-used account marker written before the exec.
- `account list` in human and verb-level JSON forms, with local usability decided from cheap metadata alone.
- The hard login-mode version-floor refusal before the exec, reusing the version-floor catalog entry and its remediation.
- The `account` verb grammar: a required subcommand, `Usage` for the bare verb and for an unrecognized one, and the wrapper help surface that names it.
- Account-discovery and login-mode entries in the doctor catalog, with their feature-owned probes and remediation.
- User-facing documentation and per-rung release gates needed to make the account MVP ready for `0.2.0`.

## Out of scope

- `account status` and `account remove`, which need the credential lock scope and the confirmation contract; slice 013 owns them.
- Token ingest, rotation, redaction, and precedence warnings; slice 013 owns them.
- Reading, copying, refreshing, synchronizing, or fingerprinting child-owned credentials; testing a path for presence is not reading it.
- Profile selection or composed settings.

## Governed by

- [AGENTS.md](../../../../AGENTS.md)
- [Session isolation](../../../explanation/session-isolation.md)
- [Accounts](../../../reference/accounts.md)
- [Doctor](../../../reference/doctor.md)
- [Process runtime](../../../reference/process-runtime.md)
- [XDG storage](../../../reference/xdg-storage.md)
- [Configuration](../../../reference/configuration.md)
- [CLI surface](../../../reference/cli-surface.md)
- [Exit codes](../../../reference/exit-codes.md)
- [Logging and output](../../../reference/logging-and-output.md)
- [Presentation](../../../reference/presentation.md)
- [Testing and quality](../../../reference/testing-and-quality.md)

## Acceptance

- When accounts are listed, the wrapper shall derive them from directories without a registry index. -> accounts::account_list_discovers_directories_without_a_registry
- When account usability is reported, the wrapper shall decide it from directory security, mode metadata, and the presence of the mode's stored artifact, and shall spawn no child. -> accounts::account_list_reports_local_usability_without_spawning_a_child
- When native login succeeds, the child shall own one shared saved login under the account configuration directory and the wrapper shall not inspect it. -> accounts::native_login_delegates_to_the_child_owned_shared_config
- When a login-mode account is selected for launch, the wrapper shall set `CLAUDE_CONFIG_DIR`, set no wrapper token variable, and record the selection before the exec. -> accounts::marker_is_written_before_the_selected_account_exec
- If a login-mode launch resolves a child below the documented floor or a version that does not parse, then the wrapper shall refuse before the exec and shall emit the catalog remediation. -> accounts::login_launch_below_the_version_floor_refuses_before_exec
- When no account is selected, the wrapper shall stop before authentication inspection or child invocation under slice 017; historical evidence remains `accounts::unselected_passthrough_never_runs_the_version_probe`.
- When the account MVP lands, its doctor catalog entries, user documentation, and `0.2.0` release gates shall describe only behavior this rung implements. -> doctor::the_published_documentation_matches_the_implemented_rung

## Rabbit holes

- Token support while the account directory is being established; escape: keep every wrapper-owned secret and lifecycle command in slice 013.
- Shipping `status` or `remove` for symmetry; escape: listing already answers selection and local usability, and the lock those two need is unimplemented.
- Wrapper-managed OAuth; escape: delegate native login to the child and never inspect its credential.

## Done when

Targeted account discovery, native login, launch environment, marker, listing, usability, version-floor refusal, doctor-catalog, and passthrough tests pass under `cargo nextest`, the `0.2.0` rung documentation is honest, and `just hooks` is green.

## Revisions

- 2026-08-12 — Slice 017 superseded only the unselected-launch behavior with mandatory pre-exec binding; the account MVP's other outcomes remain unchanged.
