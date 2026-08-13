# 022 — Account profile binding

## Goal

Make an account carry the profile it runs with, chosen out loud when the account is created, so a reader asking "which settings is this account under" gets an answer from the account itself rather than from whatever the ambient configuration happens to resolve.

## Appetite

2 implementation sessions.

## Core

An account records one profile, every account surface reports it and where it came from, and creating an account never leaves that choice implicit.

## In scope

- One decision binding a profile to an account, amending the clause in [ADR-0090](../../../decisions/ADR-0090-require-account-and-profile-before-child-launch.md) that refuses persisted profile state, and recording the rejected option of reporting only the effective profile.
- One decision on changing a binding without re-authenticating, recording the rejected option of setting it at login alone.
- A per-account durable record beside the authentication metadata, so a rebind never writes the file the token rotation sequence commits.
- One new rung in the profile resolution ladder, below the project layer and above user configuration, with its own selection spelling.
- A profile argument on `account login`, resolved from the argument, then the account's existing binding, then `default_profile`, and refused as `Config` when none of the three answers.
- A verb that rebinds an existing account, refusing a profile that has no file.
- The binding and its provenance in `account list` and `account status`, in both forms, with a bound profile whose file has gone reported as data rather than as prose.
- One account-scope `doctor` check that the selected account is bound and that its profile file exists, carrying a title, a consequence, and a next action.
- Alignment of [accounts](../../../reference/accounts.md), [configuration](../../../reference/configuration.md), [the CLI surface](../../../reference/cli-surface.md), [XDG storage](../../../reference/xdg-storage.md), and [session isolation](../../../explanation/session-isolation.md), whose independence claim this slice reverses.

## Out of scope

- Any change to how a profile composes, which pieces it takes, or how arrays merge; this slice decides which profile an account uses, never what one contains.
- Any account component in the composed entry key, which [ADR-0064](../../../decisions/ADR-0064-key-composed-settings-by-profile-and-input-digest.md) settles: two accounts bound to one profile share one entry.
- Rewriting any human report for a person, which is the next slice's whole subject and would otherwise be done twice over a moving field set.
- A migration step for accounts that predate the binding; an unbound account resolves as it does today and is reported as unbound.
- Letting a project file supply the account, which [ADR-0071](../../../decisions/ADR-0071-restrict-the-project-layer-to-the-profile-key.md) refuses on its own grounds.

## Governed by

- [AGENTS.md](../../../../AGENTS.md)
- [Charter](../../charter.md)
- [ADR-0006](../../../decisions/ADR-0006-place-files-by-xdg-ownership.md)
- [ADR-0024](../../../decisions/ADR-0024-machine-output-is-a-per-verb-flag.md)
- [ADR-0051](../../../decisions/ADR-0051-let-every-surface-element-discriminate.md)
- [ADR-0064](../../../decisions/ADR-0064-key-composed-settings-by-profile-and-input-digest.md)
- [ADR-0071](../../../decisions/ADR-0071-restrict-the-project-layer-to-the-profile-key.md)
- [ADR-0087](../../../decisions/ADR-0087-keep-the-credential-lock-beside-the-account.md)
- [ADR-0090](../../../decisions/ADR-0090-require-account-and-profile-before-child-launch.md)
- [ADR-0091](../../../decisions/ADR-0091-refuse-an-unconfigured-first-launch-without-scaffolding.md)
- [ADR-0094](../../../decisions/ADR-0094-give-every-check-a-title-and-a-next-action.md)
- [Accounts](../../../reference/accounts.md)
- [Configuration](../../../reference/configuration.md)
- [CLI surface](../../../reference/cli-surface.md)
- [XDG storage](../../../reference/xdg-storage.md)
- [Exit codes](../../../reference/exit-codes.md)
- [Doctor](../../../reference/doctor.md)
- [Coding conventions](../../../reference/coding-conventions.md)
- [Testing and quality](../../../reference/testing-and-quality.md)

## Acceptance

- When an account is created, the report shall name the profile it is bound to and the layer that supplied it. -> accounts::a_login_names_the_profile_it_bound_and_where_it_came_from
- When `account login` resolves no profile from its argument, the account, or `default_profile`, it shall refuse as `Config` and name both ways to supply one. -> accounts::a_login_with_no_profile_anywhere_is_refused_as_configuration
- When an account is rebound to a profile that has no file, the wrapper shall refuse as `NoInput` and shall leave the previous binding in place. -> accounts::binding_a_profile_without_a_document_keeps_the_previous_binding
- When a launch resolves a profile and the selected account is bound, the binding shall outrank user configuration and shall not outrank the project layer, the environment, or the flag. -> accounts::the_binding_outranks_user_configuration_and_yields_to_the_project_layer
- When an account is reported, its bound profile and the provenance of the profile in force shall appear in both the human and the machine form. -> accounts::binding_records_the_profile_and_every_account_surface_reports_it
- When a bound profile's file is absent, `account status` shall carry that as a warning value rather than as prose alone. -> accounts::a_bound_profile_with_no_document_is_carried_as_data
- When an account has no binding, every surface shall report it as unbound and no launch behaviour shall change.
- When an account is removed, its binding shall be removed with it. -> accounts::removing_an_account_takes_its_binding_with_it

## Rabbit holes

- Widening the binding into a general per-account settings store; escape: one profile name and the time it was recorded, and nothing else lives in the record.
- Keying composed entries by account so each account gets its own materialized settings; escape: [ADR-0064](../../../decisions/ADR-0064-key-composed-settings-by-profile-and-input-digest.md) already keys them by content, and identical inputs must keep sharing one file.
- Rewriting the account reports for a person while the fields are being added; escape: the fields land in today's shape and the next slice rewrites the shape once.
- Adding a second rebinding path because login already writes the account; escape: one verb changes a binding, and login resolves one when it creates an account.

## Done when

An account carries its profile, creating one says which profile and why, every account surface reports the binding and its provenance in both forms, the ladder places it under the project layer and over user configuration, the decisions are recorded against the clause they amend, and `just hooks` is green.

## Revisions

None.
