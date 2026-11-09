# 041 — Saved login presence

## Goal

Stop the wrapper from taking an MCP server sign-in for a saved login. The child keeps both in one credential file, and the wrapper took any file at that path as a login. A token account that signed in to an MCP server got a false shadowing warning on every launch. A login account with no login counted as usable, and a switch to token mode deleted MCP sign-ins that superseded nothing.

## Appetite

1 implementation session.

## Core

Every surface that asks whether an account has a saved login reads the top-level key names of the child's credential file, and no surface reads a value.

## In scope

- One probe that reads the key names of the child's credential file, never through a link, up to a fixed bound, and skips every value.
- The shadowing warning, `account status`, launch usability, the login commit, and mode-switch retirement, all through that probe.
- Retirement that keeps a file holding only MCP sign-ins, and a report line and a JSON key when a retired file took MCP sign-ins with it.
- One decision amending the rule that the wrapper never reads a child credential, and [accounts](../../../reference/accounts.md) and [XDG storage](../../../reference/xdg-storage.md) aligned with it.
- The two child key names registered in [child facts](../../../reference/child-facts.yaml) and tracked in [research tracking](../../../reference/research-tracking.yaml).

## Out of scope

- Rewriting the child's credential file to keep MCP sign-ins across a switch; the wrapper does not author child credential files ([ADR-0128](../../../decisions/ADR-0128-tell-a-saved-login-by-its-key.md)).
- Judging whether a saved login is expired or valid; the child owns that ([ADR-0088](../../../decisions/ADR-0088-model-nothing-the-child-already-owns.md)).
- A `doctor` check for MCP sign-ins; no reader acts on their presence.
- The witness question of [Q-010](../../open-questions.md); the probe narrows what counts as a credential but does not tell this run's exchange from an earlier one.

## Governed by

- [AGENTS.md](../../../../AGENTS.md)
- [Charter](../../charter.md)
- [ADR-0025](../../../decisions/ADR-0025-share-one-native-login-per-account.md)
- [ADR-0051](../../../decisions/ADR-0051-let-every-surface-element-discriminate.md)
- [ADR-0088](../../../decisions/ADR-0088-model-nothing-the-child-already-owns.md)
- [ADR-0089](../../../decisions/ADR-0089-carry-a-child-owned-fact-only-against-an-obligation.md)
- [ADR-0101](../../../decisions/ADR-0101-retire-the-credential-a-mode-switch-supersedes.md)
- [ADR-0104](../../../decisions/ADR-0104-share-one-credential-store.md)
- [ADR-0128](../../../decisions/ADR-0128-tell-a-saved-login-by-its-key.md)
- [Accounts](../../../reference/accounts.md)
- [XDG storage](../../../reference/xdg-storage.md)
- [Child facts](../../../reference/child-facts.yaml)
- [Coding conventions](../../../reference/coding-conventions.md)
- [Testing and quality](../../../reference/testing-and-quality.md)

## Acceptance

- When a token account's credential file holds only MCP sign-ins, the launch and `account status` shall raise no shadowing warning. -> accounts::mcp_sign_ins_alone_are_not_a_shadowed_saved_login
- When a token account's credential file holds a saved login, `account status` shall still report the shadowing. -> accounts::status_carries_shadowing_as_data_rather_than_prose
- When a login account's credential file holds only MCP sign-ins, the account shall be reported unusable and its launch shall refuse. -> accounts::a_login_account_with_only_mcp_sign_ins_is_not_usable
- When a switch to token mode meets a credential file that holds only MCP sign-ins, the file shall stay and the report shall retire nothing. -> accounts::a_token_login_keeps_a_file_that_holds_only_mcp_sign_ins
- When a switch to token mode retires a saved login that shares its file with MCP sign-ins, the report shall say that the sign-ins went with it. -> accounts::a_token_login_that_retires_mcp_sign_ins_says_so
- Where the credential file is malformed, the probe shall find no saved login. -> services::account::saved_login::tests::a_malformed_file_is_no_saved_login
- Where the credential file is a link, the probe shall not follow it. -> services::account::saved_login::tests::a_link_is_not_followed
- Where the credential file exceeds the bound, the probe shall find no saved login. -> services::account::saved_login::tests::a_file_past_the_limit_is_no_saved_login

## Rabbit holes

- Deserializing the login object into a typed struct; escape: the question is presence, and a typed read keeps token values in memory for no reader.
- Treating an unreadable file as a saved login to stay cautious; escape: the child cannot log in with it either, and the cautious answer is the false claim this slice removes.
- Preserving MCP sign-ins by rewriting the file; escape: the child writes the same file and coordinates its own writers, and a second writer races them.

## Done when

A file holding only MCP sign-ins raises no warning, makes no login account usable, and survives a switch to token mode, a retirement that takes MCP sign-ins says so, the decision and the references agree, both key names are registered, and `just hooks` is green.

## Revisions

- None.
