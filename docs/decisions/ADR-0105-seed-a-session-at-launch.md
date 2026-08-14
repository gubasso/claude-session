# ADR-0105: Answer a session directory's first-run questions at launch

## Context and Problem Statement

[ADR-0098](./ADR-0098-seed-the-one-child-key-a-launch-cannot-reach.md) writes `hasCompletedOnboarding` once, at login, into the account's configuration directory, because the wrapper created that directory and its newness is what makes the child ask. [ADR-0102](./ADR-0102-key-child-state-by-terminal.md) moves the file the child reads into a per-terminal directory that does not exist at login and is created on first launch. The seed no longer reaches the file it was written for, and every new terminal meets first-run setup instead of a prompt. Workspace trust has the same shape: recorded per configuration directory, it resets in every terminal, for every project.

## Considered Options

- Seed at login for every terminal that might later exist.
- Leave both questions to the user, once per terminal, per project.
- Seed at launch, into the directory the launch just created.

## Decision Outcome

Chosen option: seed at launch. The write happens when the wrapper materialises a session directory, under the account's credential lock and the atomic sequence, refusing a file that is not an object — the mechanism ADR-0098 already specifies, moved to the moment the directory exists.

Two keys, not one. `hasCompletedOnboarding` is unconditional and carries ADR-0098's reasoning unchanged: the wrapper created the directory that makes the child ask. The trust key is written only for the launch directory and only while `auto_trust_cwd` is enabled, because marking a directory trusted is a decision about code execution and the user must be able to decline it. It defaults to enabled, since the alternative is re-approving every project in every terminal.

Seeding at login would require predicting terminals. Leaving both makes isolation cost more than it returns.

## Consequences

- Good: a new terminal reaches the prompt, in the directory that launch created.
- Bad: the wrapper writes a second key inside a file the child otherwise owns.
- Bad: a default-on trust seed means the wrapper answers a security question on the user's behalf, which the configuration key exists to let them take back.

## Status

Implemented

Amends [ADR-0098](./ADR-0098-seed-the-one-child-key-a-launch-cannot-reach.md). Enacted in [the readiness service](../../src/services/account/onboarding.rs). Shaped by [028](../plan/slices/028-per-terminal-session-isolation/README.md).
