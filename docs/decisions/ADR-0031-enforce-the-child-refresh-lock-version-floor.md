# ADR-0031: Enforce the child refresh-lock version floor

## Context and Problem Statement

Shared-login mode depends on the child's cross-process refresh coordination introduced in version 2.1.211. The wrapper must decide what a launch does when the child is older or its version cannot be parsed.

## Considered Options

- Hard-fail a login-mode launch.
- Warn and proceed without the required safety guarantee.
- Degrade by disabling shared-login mode.

## Decision Outcome

Chosen option: **hard-fail a login-mode launch** below the floor. The wrapper refuses to spawn, reports the detected version, the required 2.1.211, and how to upgrade.

An unparsable version is treated as below the floor: the check fails closed, because the failure it prevents is a silent account-wide logout while the failure it causes is loud and locally fixable.

Enforcement is scoped to what actually depends on the lock:

- **Blocked** — a `login`-mode launch, which shares one saved login across concurrent runs.
- **Not blocked** — `token` mode, which injects a credential the child never refreshes; and any passthrough with no account selected, which the wrapper leaves alone under [ADR-0002](./ADR-0002-verbatim-argv-passthrough.md).

The existing soft `doctor` probe reports version state but is voluntary, so it does not satisfy this launch-time precondition.

## Consequences

- Good: the revocation storm [ADR-0025](./ADR-0025-share-one-native-login-per-account.md) depends on avoiding cannot occur silently.
- Good: shared-login mode is unblocked for implementation.
- Bad: an old child blocks login mode outright; the message must make the upgrade obvious.
- Bad: the wrapper must detect the child's version before every login-mode launch.

See [process runtime](../reference/process-runtime.md), [prior art](../reference/prior-art.md), and [research tracking](../reference/research-tracking.yaml).

## Status

Accepted
