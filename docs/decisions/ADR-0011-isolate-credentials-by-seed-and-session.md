# ADR-0011: Isolate credentials by per-account seed and per-session copy

## Context and Problem Statement

Several terminals must run as different accounts at once, and the child stores credentials in whatever configuration directory it is pointed at. Sharing one credential file between concurrent sessions lets one session's token refresh invalidate another's, and makes the active account a global mode rather than a property of the terminal.

## Considered Options

- One shared credential store per account, referenced by every session.
- Symbolic links from each session directory into a shared store.
- A per-account seed, copied into each session directory at session start.

## Decision Outcome

Chosen option: seed plus per-session copy — a copy gives each session an independent file the child can rewrite freely, without a concurrent session observing a half-written state or losing a refresh.

Authentication runs the child's own login in a scratch directory and copies the result into the account's seed. Symlinks are rejected: they let one session's write reach another and behave badly under bind mounts.

Subscription login is the primary and always-available path; an injected API token is secondary, for headless and continuous-integration use. The two are never mixed within one invocation.

Credentials live only in secured state directories — never in user-editable configuration, cache, or the runtime directory. Files are private to the user, written atomically, and read only after ownership and symlink checks. See [XDG storage](../reference/xdg-storage.md) and [session isolation](../explanation/session-isolation.md).

## Consequences

- Good: concurrent sessions on one account cannot corrupt each other's credential state.
- Good: an account is a property of the terminal, not a machine-wide mode a second terminal can silently change.
- Bad: a credential exists in more than one place, so a revoked account must be cleaned from every live session and stale copies pruned.
- Bad: a token refreshed inside a session does not propagate back, so a session can hold a newer credential than its account.
- Bad: login needs a browser, so container use falls back to the token path — which must stay first-class, not an afterthought.

## Status

Superseded

Superseded by [ADR-0025](./ADR-0025-share-one-native-login-per-account.md).
