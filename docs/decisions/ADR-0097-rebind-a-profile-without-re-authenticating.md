# ADR-0097: Rebind a profile without re-authenticating

## Context and Problem Statement

[ADR-0096](./ADR-0096-bind-a-profile-to-an-account.md) makes `account login` establish an account's profile. Changing that choice later is a configuration edit, but login is the only writer of the account, and login means a browser flow or a pasted token.

## Considered Options

- Add one verb that changes the binding alone.
- Change it only by running `account login` again.
- Edit the per-account file by hand.

## Decision Outcome

Chosen option: one verb — the cost of the alternative is a credential ceremony charged for a settings change, which is the kind of friction that teaches users to edit state files instead.

`account bind <name> --profile <name>` writes the binding under the account lock and nothing else. It validates that the profile has a file first and refuses as `NoInput` otherwise, leaving the previous binding in place, so a typo never unbinds a working account. It creates no account: an unknown name is the same `NoInput` the other account verbs raise. It declares its own `--json` like every verb that produces data.

Hand-editing stays possible and stays unsupported. The file is the wrapper's, its shape is not a published contract, and a verb that validates the name is the difference between a refusal now and a failed launch later.

## Consequences

- Good: changing settings costs a settings command.
- Good: the binding has one writer that validates, rather than two that differ.
- Bad: one more name claimed inside the `account` namespace.

## Status

Implemented

Extends [ADR-0096](./ADR-0096-bind-a-profile-to-an-account.md). The claimed name is inside the wrapper's own `account` namespace, so [ADR-0003](./ADR-0003-reserve-a-small-wrapper-cli-surface.md) is unaffected. Enacted by [the bound profile](../reference/accounts.md#the-bound-profile). Shaped by [022](../plan/slices/022-account-profile-binding/README.md).
