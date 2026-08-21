# ADR-0113: Key a session to its running agent

## Context and Problem Statement

[ADR-0102](./ADR-0102-key-child-state-by-terminal.md) keyed child state by terminal. A terminal is a reusable slot whose name is issued per namespace, and it exists whether or not anything runs in it. Every liveness verdict but `live` traced back to that key, and a directory outlived the thing it stood for.

## Considered Options

- Keep the terminal key and go on judging it.
- Key by an identifier the wrapper mints per launch.
- Key by the agent process itself.

## Decision Outcome

Chosen option: the agent process. A session is one running coding agent and nothing else. The wrapper execs the child, so the process that becomes the agent is this one; its identifier and start time, read before the exec, name the directory and answer the liveness question after it. The process namespace scopes the name, because that is what issued the identifier.

Liveness becomes one question — is that process still running under the boot the record names — and the naming ladder, the device mapping, and the alias ground go with the terminal. A record from the terminal-keyed version is not read, so the directory it named is collectable.

A session now lasts one agent run, so a launch first collects that account's provably dead sessions. Without it, one directory per launch would grow without bound between explicit collections.

A minted identifier was rejected: it would need a liveness record of its own, and the process already is one.

## Consequences

- Good: every verdict traces to a fact about a process, and no directory outlives what it names.
- Good: a session directory is fresh, so nothing can already occupy a name inside it.
- Bad: nothing in that directory carries across runs, so the child's prompt history and the keys it writes start empty each launch. Transcripts and peer registrations are unaffected, being shared trees the session links to.

## Status

Implemented

Supersedes [ADR-0102](./ADR-0102-key-child-state-by-terminal.md). Amends [ADR-0107](./ADR-0107-scope-a-terminal-to-its-namespace.md) and [ADR-0110](./ADR-0110-record-the-terminal-witness-at-launch.md). Enacted in [agent naming](../../src/domain/agent.rs), [the judgment](../../src/domain/witness.rs), and [the collector](../../src/services/session/gc.rs). Shaped by [036](../plan/slices/036-agent-keyed-sessions/README.md).
