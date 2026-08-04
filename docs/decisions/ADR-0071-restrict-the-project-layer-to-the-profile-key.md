# ADR-0071: Restrict the project layer to the profile key

## Context and Problem Statement

[ADR-0070](./ADR-0070-discover-the-project-configuration-file-at-the-repository-root.md) gives a repository a configuration file the wrapper reads before doing anything. A cloned repository is untrusted content. If that file may set every key, then `git clone && cd && claude-session` lets the repository choose which executable runs as the child and which stored credential it runs under, before the user has read a line of it.

## Considered Options

- Every key is settable at every layer, symmetric and simple to explain.
- Restrict the project layer to keys that cannot select code or credentials.
- Keep every key but require an interactive trust prompt on first use of a project file.

## Decision Outcome

Chosen option: **the project layer may set `default_profile` and nothing else.** `child_bin` selects an executable; `default_account` selects a credential. Neither belongs to a file that arrives with the source tree. `default_profile` names a profile the user authored under their own config base — a repository can ask for one, and gets nothing if it does not exist.

`child_bin` or `default_account` appearing in a project file is `Config`, named and rejected, not silently ignored: the wrapper's [unknown-key rule](../reference/configuration.md#schema) already holds that a configuration which does not do what it says is the worse failure.

A trust prompt was rejected as the wrong shape for this project. The wrapper is a launcher whose non-interactive path must stay scriptable, and a prompt answered once permanently would be a stored decision with no home in the [XDG layout](../reference/xdg-storage.md).

## Consequences

- Good: cloning a repository can never redirect the child binary or the account.
- Good: the restriction is one column in the key table, not a separate mechanism.
- Bad: a per-repository child binary — a monorepo pinning its own `claude` — needs the environment or a flag instead.
- Bad: the layer table is no longer uniform, so each new key must decide its layers.

## Status

Accepted

Constrains [ADR-0070](./ADR-0070-discover-the-project-configuration-file-at-the-repository-root.md). The eligible layers per key are in [the key table](../reference/configuration.md#keys).
