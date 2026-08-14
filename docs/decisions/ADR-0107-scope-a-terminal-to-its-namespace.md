# ADR-0107: Scope a terminal to the namespace that issued it

## Context and Problem Statement

[ADR-0102](./ADR-0102-key-child-state-by-terminal.md) names the child state directory after the terminal, which is unique on one machine. A bind-mounted state tree is not one machine: every container carries its own devpts, so the first shell in each is `/dev/pts/0` and derives `pts-0`. Two containers and their host then write one `.claude.json`, which is the collision ADR-0102 exists to prevent, returning silently because the directories look separate.

## Considered Options

- Fold a namespace tag into the terminal identifier.
- Give the namespace its own path component.
- Refuse to run when the state tree is reachable from another namespace.

## Decision Outcome

Chosen option: its own path component — `accounts/<account>/sessions/<namespace>/<terminal>/`. Namespace and terminal are separate axes, and folding them would leave the longest session-leader name one byte inside the identifier limit.

The tag is read per rung, because each rung's identifier belongs to a different namespace: the mount namespace issues devpts names, the PID namespace issues session ids. `docker run --pid=host` is the reachable case where one uniform tag would collide and these do not.

A rung whose namespace cannot be read yields nothing, so the ladder falls through and ADR-0102's refusal absorbs it. Producing the undiscriminated name instead would knowingly restore the defect.

Refusing on a shared tree was rejected: the wrapper cannot see who else mounts it, and the sharing is wanted.

## Consequences

- Good: a container and its host stop sharing child state, and each tag names the namespace that owns its rung.
- Good: the credential store, the projects tree, and the declared link are untouched, so sharing them stays correct.
- Bad: existing session directories become unreachable, and nothing prunes them.
- Bad: a namespace is not stable across container recreation, so a recreated container starts a fresh session directory.

## Status

Implemented

Amends [ADR-0102](./ADR-0102-key-child-state-by-terminal.md): the derivation gains an axis and the path gains a component, while keying only the child state directory is unchanged. Enacted in [the namespace](../../src/domain/namespace.rs), [the ladder](../../src/domain/terminal.rs), and [the terminal adapter](../../src/adapters/terminal.rs). Shaped by [030](../plan/slices/030-namespace-scoped-terminal-identity/README.md).
