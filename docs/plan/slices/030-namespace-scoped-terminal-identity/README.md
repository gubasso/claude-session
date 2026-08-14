# 030 — Namespace-scoped terminal identity

## Goal

Stop a terminal named in one kernel namespace from naming another namespace's state directory, so a state tree shared into containers keeps the separation [028](../028-per-terminal-session-isolation/README.md) built.

## Appetite

1 implementation session.

## Core

Two runs that share the state tree but not their namespaces receive different session directories.

## In scope

- One decision scoping the terminal to the namespace that issued it, amending [ADR-0102](../../../decisions/ADR-0102-key-child-state-by-terminal.md) rather than replacing it, since keying only the child state directory is unchanged.
- A namespace component above the terminal under `sessions/`, rather than a tag folded into an identifier whose grammar leaves no room for one.
- A per-rung derivation, because the mount namespace issues the device names the first rung reads and the PID namespace issues the session ids the second one reads.
- An unreadable namespace making its rung unavailable, so the ladder's existing refusal absorbs it and no undiscriminated name is ever produced.
- The `session-terminal-derives` report naming the namespace beside the terminal and the rung, under [ADR-0018](../../../decisions/ADR-0018-one-probe-set-with-stable-check-ids.md), without a new check id.
- Alignment of [XDG storage](../../../reference/xdg-storage.md), [accounts](../../../reference/accounts.md), [doctor](../../../reference/doctor.md), and [session isolation](../../../explanation/session-isolation.md).

## Out of scope

- Migrating or pruning the session directories the rename strands, which [ADR-0102](../../../decisions/ADR-0102-key-child-state-by-terminal.md) already records as unpruned and which carry nothing a launch cannot rebuild.
- The credential store and the declared link, which stay exactly as [ADR-0103](../../../decisions/ADR-0103-permit-a-declared-link.md) and [ADR-0104](../../../decisions/ADR-0104-share-one-credential-store.md) left them; sharing them across namespaces is the wanted behaviour.
- Detecting that a state tree is shared at all, which the wrapper cannot see and does not need to, because the namespace answers the question without asking it.
- The devcontainer tooling that made the collision reachable, which is another repository and would breach the self-containment rule in [AGENTS.md](../../../../AGENTS.md).
- Any new configuration key, check id, or carried child fact; this slice adds none.

## Governed by

- [AGENTS.md](../../../../AGENTS.md)
- [Charter](../../charter.md)
- [ADR-0006](../../../decisions/ADR-0006-place-files-by-xdg-ownership.md)
- [ADR-0018](../../../decisions/ADR-0018-one-probe-set-with-stable-check-ids.md)
- [ADR-0046](../../../decisions/ADR-0046-support-linux-and-a-single-child-baseline.md)
- [ADR-0048](../../../decisions/ADR-0048-build-for-a-present-need.md)
- [ADR-0051](../../../decisions/ADR-0051-let-every-surface-element-discriminate.md)
- [ADR-0102](../../../decisions/ADR-0102-key-child-state-by-terminal.md)
- [ADR-0103](../../../decisions/ADR-0103-permit-a-declared-link.md)
- [ADR-0107](../../../decisions/ADR-0107-scope-a-terminal-to-its-namespace.md)
- [XDG storage](../../../reference/xdg-storage.md)
- [Doctor](../../../reference/doctor.md)
- [Coding conventions](../../../reference/coding-conventions.md)
- [Testing and quality](../../../reference/testing-and-quality.md)
- [Development workflow](../../../guides/development-workflow.md)

## Acceptance

- When two runs differ only by the namespace that named them, each shall receive its own session directory. -> domain::terminal::tests::one_device_in_two_namespaces_names_two_terminals
- Where a namespace names nothing, the rung reading it shall name nothing rather than fall back to a shared name. -> domain::namespace::tests::an_empty_link_names_nothing
- When a terminal is reported, the report shall name the namespace beside the terminal and the rung it came from. -> doctor::the_terminal_report_names_the_namespace
- When the work lands, no document shall describe a session directory without its namespace component.

## Rabbit holes

- Detecting containers, which is a heuristic answering a question the namespace answers exactly; escape: read the namespace and never ask what it means.
- Making the namespace stable across a container's recreation, which nothing on the machine records; escape: a recreated container is a new machine, and a fresh directory is the honest result.
- Migrating stranded directories at launch, which cannot know which namespace wrote one; escape: they were already unpruned, and the launch rebuilds what matters.
- Reaching for a longer identifier to fit a folded tag; escape: the grammar is shared with accounts and profiles, and the namespace is its own axis anyway.

## Done when

A session directory sits under the namespace that named its terminal, two namespaces sharing one state tree never share one directory, the report names both, and `just hooks` is green.

## Revisions

None.
