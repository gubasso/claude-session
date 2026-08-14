# 026 — Refresh-token bootstrap

## Goal

Give a machine with no browser a way to reach a full saved login. `claude setup-token` mints an inference-only credential, so a token account cannot report its subscription, its organization, or its usage however much the wrapper injects — the scope is refused at the server. The child accepts a refresh token instead and does the whole exchange itself, which is the one non-interactive path that ends in the same credential a browser login leaves.

## Appetite

2 implementation sessions.

## Core

One `account login --refresh-token --stdin` on a terminal-less machine leaves an account a launch reaches as the subscription it is, with nothing of the refresh token kept.

## In scope

- A `--refresh-token` selector on `account login`, taking its secret from standard input or the controlling terminal under [ADR-0027](../../../decisions/ADR-0027-ingest-secrets-only-from-stdin-or-a-terminal.md), never from argv.
- A `--scopes` option, defaulting to the set a claude.ai login is issued with, because the child requires the scopes the refresh token was issued with and refuses without them.
- Delegation of the exchange to the child's own `auth login`, in the account's configuration directory, with the two variables the child documents for it.
- Commit through the existing saved-login path, so the account records `login` mode and nothing new is stored.
- One decision recording why a refresh token is spent rather than kept, and why this adds no mode.
- Registration of both carried variables in [child facts](../../../reference/child-facts.yaml), and of the default scope set in [research tracking](../../../reference/research-tracking.yaml).
- Alignment of [accounts](../../../reference/accounts.md) and [the CLI surface](../../../reference/cli-surface.md).

## Out of scope

- Storing the refresh token. It may be rotated by the exchange, so a stored copy could be dead with nothing local able to tell.
- Obtaining a refresh token. The wrapper does not mint one and does not read one out of the child's credential file; the operator supplies it.
- A third stored mode. The artifact and every later behaviour are a saved login's, and a distinction that changes no reader's action does not earn a surface ([ADR-0051](../../../decisions/ADR-0051-let-every-surface-element-discriminate.md)).
- Verifying the declared scopes against the token. The exchange is the verification, and the child reports its own failure.
- Repairing token mode. It stays inference-only automation with a declared plan ([ADR-0099](../../../decisions/ADR-0099-declare-the-plan-a-token-cannot-carry.md)).

## Governed by

- [AGENTS.md](../../../../AGENTS.md)
- [Charter](../../charter.md)
- [ADR-0011](../../../decisions/ADR-0011-isolate-credentials-by-seed-and-session.md)
- [ADR-0021](../../../decisions/ADR-0021-fail-closed-without-a-terminal.md)
- [ADR-0025](../../../decisions/ADR-0025-share-one-native-login-per-account.md)
- [ADR-0026](../../../decisions/ADR-0026-store-and-inject-a-long-lived-subscription-token.md)
- [ADR-0027](../../../decisions/ADR-0027-ingest-secrets-only-from-stdin-or-a-terminal.md)
- [ADR-0030](../../../decisions/ADR-0030-use-account-login-for-wrapper-authentication.md)
- [ADR-0051](../../../decisions/ADR-0051-let-every-surface-element-discriminate.md)
- [ADR-0057](../../../decisions/ADR-0057-build-the-child-environment-by-prefix-scrub-and-marker.md)
- [ADR-0068](../../../decisions/ADR-0068-spawn-the-child-as-a-subroutine.md)
- [ADR-0088](../../../decisions/ADR-0088-model-nothing-the-child-already-owns.md)
- [ADR-0089](../../../decisions/ADR-0089-carry-a-child-owned-fact-only-against-an-obligation.md)
- [ADR-0093](../../../decisions/ADR-0093-write-every-non-machine-surface-for-a-person.md)
- [ADR-0096](../../../decisions/ADR-0096-bind-a-profile-to-an-account.md)
- [ADR-0098](../../../decisions/ADR-0098-seed-the-one-child-key-a-launch-cannot-reach.md)
- [ADR-0099](../../../decisions/ADR-0099-declare-the-plan-a-token-cannot-carry.md)
- [Accounts](../../../reference/accounts.md)
- [Child facts](../../../reference/child-facts.yaml)
- [CLI surface](../../../reference/cli-surface.md)
- [Research tracking](../../../reference/research-tracking.yaml)
- [XDG storage](../../../reference/xdg-storage.md)
- [Process runtime](../../../reference/process-runtime.md)
- [Presentation](../../../reference/presentation.md)
- [Coding conventions](../../../reference/coding-conventions.md)
- [Testing and quality](../../../reference/testing-and-quality.md)

## Acceptance

- When `account login --refresh-token --stdin` succeeds, the wrapper shall record the account as a saved login. -> accounts::a_refresh_login_records_a_saved_login
- When that login runs, the refresh token shall reach the child in its environment and appear in no argument. -> accounts::a_refresh_login_carries_the_secret_in_the_environment_only
- When no `--scopes` is given, the wrapper shall hand the child the scope set a claude.ai login is issued with. -> accounts::a_refresh_login_without_scopes_uses_the_set_a_login_is_issued
- When `--scopes` carries a value, the wrapper shall hand the child that value instead. -> accounts::a_declared_scope_set_replaces_the_default
- When `--scopes` carries a value outside the shape, the wrapper shall refuse before running the child. -> accounts::a_malformed_scope_set_is_refused_before_the_child_runs
- When a refresh-token login runs with `--stdin` and no controlling terminal, the wrapper shall complete it rather than refuse. -> accounts::a_refresh_login_needs_no_terminal
- When the child leaves no saved login, the wrapper shall fail the login and remove the account it created. -> accounts::a_refresh_login_that_leaves_no_saved_login_removes_the_account
- When a refresh-token login succeeds, no file under the account shall hold the refresh token. -> accounts::a_refresh_token_is_never_written_under_the_account
- When `--token` and `--refresh-token` are given together, the wrapper shall refuse as a usage error. -> accounts::a_token_and_a_refresh_token_together_are_refused

## Rabbit holes

- Storing the refresh token so a later login can reuse it; escape: the exchange may rotate it, and a credential nothing can validate locally is worse than no credential.
- Reading the refresh token out of the child's `.credentials.json`; escape: the wrapper does not read child credentials, and naming where one lives is prose the operator acts on.
- Adding a third stored mode, a metadata field, or a `doctor` check for how the login was obtained; escape: the account is a saved login, and every existing surface already reports one.
- Speaking the token endpoint to exchange the refresh token; escape: the child does it, which is what keeps [ADR-0026](../../../decisions/ADR-0026-store-and-inject-a-long-lived-subscription-token.md) intact.
- Inventing a second child-version floor for the branch that reads these variables; escape: the commit gate is the saved login the child leaves, so a child that ignored them commits nothing, and the unknown first version is tracked rather than guessed.

## Done when

A refresh-token login on a machine with no terminal leaves an account whose launch the child describes as the subscription it is, the refresh token is nowhere under the account directory, both carried variables are registered against their obligation and the default scope set is tracked as perishable, and `just hooks` is green.

## Revisions

- None.
