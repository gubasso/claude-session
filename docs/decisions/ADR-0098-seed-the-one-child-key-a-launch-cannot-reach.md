# ADR-0098: Seed the one child key a launch cannot reach

## Context and Problem Statement

Each account owns a fresh `CLAUDE_CONFIG_DIR` that nothing seeds. The child decides its first-run onboarding from `hasCompletedOnboarding` in that directory and never consults authentication, so a token-mode account the wrapper just authenticated opens a browser sign-in it does not need and cannot complete into its stored mode. The account is unusable while every wrapper surface reports it healthy.

## Considered Options

- Leave the file untouched and document the extra sign-in.
- Perform a second child login during `account login` so the child writes its own state.
- Write the single key at the end of a successful login, and nothing else.

## Decision Outcome

Chosen option: write the single key. The wrapper created the directory that makes the child ask, so answering that one question is part of launching the child rather than modelling it. A second login does not help: the child's own `auth login` leaves the key unset, and in token mode it would store a credential the injected token then shadows.

The write is read-modify-write under the account's credential lock and the atomic sequence, refusing a file that is not an object. It happens at login, when a child of that account is least likely to run; a launch only reads, warns, and proceeds.

## Consequences

- Good: one login yields an account a bare launch reaches the prompt with.
- Good: trust, theme, and history stay child-owned, so the workspace trust prompt still fires.
- Bad: the wrapper now writes one key inside a file the child otherwise owns, a child that renames it needs revisiting, and a child running concurrently can lose a write, because no lock reaches past the exec ([Q-009](../plan/open-questions.md)).

## Status

Implemented

Amends [ADR-0088](./ADR-0088-model-nothing-the-child-already-owns.md), which forbids modelling child-owned state; one key written against the launch obligation of [ADR-0089](./ADR-0089-carry-a-child-owned-fact-only-against-an-obligation.md) is the exception, and the key is registered in [child facts](../reference/child-facts.yaml). [ADR-0015](./ADR-0015-retire-the-init-verb.md) is untouched: this is child state inside a wrapper-owned account directory, not the user's configuration. Enacted in [the readiness service](../../src/services/account/onboarding.rs) and [accounts](../reference/accounts.md#logging-in). Shaped by [024](../plan/slices/024-launch-ready-after-login/README.md).
