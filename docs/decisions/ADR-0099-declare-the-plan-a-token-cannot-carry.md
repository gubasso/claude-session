# ADR-0099: Declare the plan an injected token cannot carry

## Context and Problem Statement

The child reads which subscription a session belongs to from the credential it saved for itself. An injected token short-circuits that read and supplies no plan, so the child treats the tier as unknown: it describes the session as an API one and falls back to its no-plan default model. The account is on a subscription throughout, and only the wrapper that replaced the credential can say which one.

## Considered Options

- Leave it, and document that token mode reports the wrong tier.
- Read the plan from the child, or from the service that issued the token.
- Have the user declare the plan at login, and inject it.

## Decision Outcome

Chosen option: declare it at login. The child exposes the plan through no surface a wrapper can read, and asking the issuer means speaking an endpoint [ADR-0026](./ADR-0026-store-and-inject-a-long-lived-subscription-token.md) refuses. So the value is the user's, obtained by a prompt or `--plan`, validated for shape but never against a vocabulary, and written by the metadata rename that commits the token. A rotation that does not re-declare clears it, so a declaration cannot outlive its credential.

Nothing is enforced. A login with nowhere to ask, and every account predating this, declare none; a launch then sets no variable and warns.

## Consequences

- Good: one token login yields a session the child describes as the subscription it is, with that subscription's default model.
- Good: the plan is bound to the token it was declared with rather than to the account's whole life.
- Bad: the wrapper asserts a tier nothing verifies, so a plan that changes mid-token over-claims until the next login; every surface words it as a declaration.
- Bad: one more child-owned variable is carried, and the spellings the prompt offers are the child's to change.

## Status

Implemented

Carried against the launch obligation of [ADR-0089](./ADR-0089-carry-a-child-owned-fact-only-against-an-obligation.md), which is what keeps it inside [ADR-0088](./ADR-0088-model-nothing-the-child-already-owns.md); registered in [child facts](../reference/child-facts.yaml) and, being undocumented upstream, in [research tracking](../reference/research-tracking.yaml). Enacted in [accounts](../reference/accounts.md#logging-in). Shaped by [025](../plan/slices/025-declared-subscription-plan/README.md).
