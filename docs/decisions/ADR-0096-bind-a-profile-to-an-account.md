# ADR-0096: Bind a profile to an account

## Context and Problem Statement

An account and a profile are resolved by two independent ladders, so nothing records which settings an account runs under and no report can answer it. [ADR-0090](./ADR-0090-require-account-and-profile-before-child-launch.md) requires both selections at launch but keeps the profile ambient, which leaves the pairing to whatever the environment resolved at that moment.

## Considered Options

- Bind a profile to the account durably, chosen when the account is created.
- Keep the two independent and report only the profile that would be in force.
- Key composed settings by account as well as by profile.

## Decision Outcome

Chosen option: bind durably — an account is the durable identity a user manages, so the settings it runs under belong to it rather than to the ambient configuration.

The binding is one profile name and the time it was recorded, in its own per-account file beside the authentication metadata, so a rebind never writes the file the token rotation sequence commits. `account login` resolves the profile from its argument, then the existing binding, then `default_profile`, refusing as `Config` when none answers, and always naming which of the three supplied it. The binding takes one rung in the profile ladder, below the project layer and above user configuration: it is the account's own intent, and a project file is a deliberate per-tree override. An account with no binding resolves exactly as before.

Composed entries are unchanged. [ADR-0064](./ADR-0064-key-composed-settings-by-profile-and-input-digest.md) keys them by profile and input digest, so two accounts bound to one profile keep sharing one entry.

## Consequences

- Good: every account surface can say which settings the account uses and why.
- Good: the pairing survives a new shell, a new tree, and an empty environment.
- Bad: one more durable artifact per account, and one more rung to explain.

## Status

Implemented

Amends [ADR-0090](./ADR-0090-require-account-and-profile-before-child-launch.md), whose requirement stands while its refusal of persisted profile state does not. [ADR-0015](./ADR-0015-retire-the-init-verb.md) is untouched: it rejected a project-to-profile binding, not an account-to-profile one. Enacted by [the bound profile](../reference/accounts.md#the-bound-profile) and [the profile ladder](../reference/configuration.md#selecting-the-active-profile). Shaped by [022](../plan/slices/022-account-profile-binding/README.md).
