# ADR-0104: Share one credential store across session directories

## Context and Problem Statement

[ADR-0025](./ADR-0025-share-one-native-login-per-account.md) requires one saved login per account, because the child rotates and revokes on refresh and coordinates that across processes through one file. [ADR-0102](./ADR-0102-key-child-state-by-terminal.md) gives each terminal its own `CLAUDE_CONFIG_DIR`, and the saved login lives inside whatever that variable points at. Taken together they contradict: N directories would hold N credentials, which is the concurrent-refresh failure ADR-0025 exists to prevent and which the shell predecessor reproduces by copying the file under a lock.

## Considered Options

- Copy the credential into each session directory and reconcile on exit.
- Link the credential file from each session directory.
- Set the child's own credential-store variable to the account directory.

## Decision Outcome

Chosen option: set the variable. The child resolves its credential store from `CLAUDE_SECURESTORAGE_CONFIG_DIR` when that name is present, and from the configuration directory only when it is absent. The wrapper sets it to `accounts/<account>/config/`, so every terminal of one account reads and writes the one file, and the child's own cross-process coordination sees exactly the single file it was written for.

The variable is undocumented, so it is carried against the launch obligation of [ADR-0089](./ADR-0089-carry-a-child-owned-fact-only-against-an-obligation.md), registered in [child facts](../reference/child-facts.yaml), and tracked in [research tracking](../reference/research-tracking.yaml). Three behaviours read from the shipped resolver are load-bearing: the value is normalized but never canonicalized or tilde-expanded, an empty value differs from an absent one, and a stale inherited value redirects a login. The wrapper therefore sets an absolute, already-resolved path and never an empty one.

Copying and linking both put a second name on one credential, which is what ADR-0025 forbids.

## Consequences

- Good: ADR-0025 survives a per-terminal layout unchanged, with no copy and no lock of the wrapper's own.
- Good: existing accounts need no migration; the credential stays where it already is.
- Bad: the wrapper now depends on a name the child does not document, which can move without notice.

## Status

Implemented

Complements [ADR-0025](./ADR-0025-share-one-native-login-per-account.md). Enacted in [the launch environment](../../src/services/child.rs). Shaped by [028](../plan/slices/028-per-terminal-session-isolation/README.md).
