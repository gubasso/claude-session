# ADR-0100: Bootstrap a saved login from a refresh token

## Context and Problem Statement

A machine with no browser can only reach token mode, and the token `setup-token` mints is inference-only by design: the server refuses it the profile scope, so such a session cannot report its subscription, its account, or its usage. Only a full claude.ai grant answers those, and nothing the wrapper injects substitutes for one.

## Considered Options

- Leave it, and document token mode's ceiling.
- Widen the injected token's declared scopes.
- Write the child's credential file from a supplied grant.
- Hand a refresh token to the child's own login and let it do the exchange.

## Decision Outcome

Chosen option: hand it to the child. `account login --refresh-token` reads one secret under [ADR-0027](./ADR-0027-ingest-secrets-only-from-stdin-or-a-terminal.md) and runs the child's `auth login` with it, plus its scopes, in the account's configuration directory. The child exchanges, stores, and describes the result; the wrapper inspects none of it and commits what the child leaves, as a native login does.

So this adds no mode. The account records `login`, because the artifact and every later behaviour are a saved login's ([ADR-0025](./ADR-0025-share-one-native-login-per-account.md)), and a distinction no reader acts on earns no surface ([ADR-0051](./ADR-0051-let-every-surface-element-discriminate.md)).

The refresh token is spent rather than stored. The exchange may rotate it, and a stored copy could then be dead with nothing local able to tell.

## Consequences

- Good: a terminal-less machine reaches the credential token mode cannot, with the wrapper still never speaking an OAuth endpoint ([ADR-0026](./ADR-0026-store-and-inject-a-long-lived-subscription-token.md)).
- Good: the subscription, the tier, and the first-run key all come from the exchange, so nothing is declared and nothing is guessed.
- Bad: the operator must obtain a refresh token, which only a completed login issues, and supply it again to repair the account.
- Bad: the scope set the wrapper defaults to is the child's to change.

## Status

Implemented

Carried against the launch obligation of [ADR-0089](./ADR-0089-carry-a-child-owned-fact-only-against-an-obligation.md), which keeps it inside [ADR-0088](./ADR-0088-model-nothing-the-child-already-owns.md); both variables are registered in [child facts](../reference/child-facts.yaml) and the default scope set in [research tracking](../reference/research-tracking.yaml). Enacted in [accounts](../reference/accounts.md#refresh-token-bootstrap). Shaped by [026](../plan/slices/026-refresh-token-bootstrap/README.md).
