# Documentation

This is the index into `claude-session`'s engineering documentation. Every durable fact lives in the document that owns it; the shape of the tree — which zones exist, and why one holds fewer pages than a reader might expect — is the one thing this file states on its own authority.

The tree is organized by **reader need** first and topic second. Four zones, four promises:

| Zone                          | What it promises the reader                                  | The question it answers     |
| ----------------------------- | ------------------------------------------------------------ | --------------------------- |
| [Decisions](./decisions/)     | The choice, its serious alternatives, and its consequences   | Why is it this way?         |
| [Explanation](./explanation/) | A mental model of a subsystem and the forces that shaped it  | How does this fit together? |
| [Reference](./reference/)     | Exact, lookup-oriented contracts — tables, matrices, schemas | What is the precise value?  |
| [Guides](./guides/)           | An ordered task with prerequisites and verification          | What do I do next?          |

A topic directory, when volume ever demands one, goes **inside** a zone. It is never a sibling of the zones, because a topic-first tree forces every reader to open a file to learn what kind of document it is.

`claude-session` is pre-implementation. These documents are normative design for the code that has not been written yet, not a description of shipped behaviour.

## Start here

- New to the project? Read [the architecture](./explanation/architecture.md), then [the wrapper model](./explanation/wrapper-model.md).
- About to write code? Read [the development workflow](./guides/development-workflow.md).
- Looking for a rule you must not break? It is in a reference page or an ADR, never in prose elsewhere.

## Explanation

- [Architecture](./explanation/architecture.md) — the crate's shape, the invocation lifecycle, and what each module may and may not do.
- [Wrapper model](./explanation/wrapper-model.md) — what it means to wrap `claude`, and why the wrapper's grammar stays small and the child stays opaque.
- [Session isolation](./explanation/session-isolation.md) — what a session is, how its identity is derived, and why redirecting the child's configuration directory is the isolation mechanism.
- [Testing strategy](./explanation/testing-strategy.md) — how this project tests a process-spawning wrapper, and the anti-patterns review rejects.

The zone carries cross-cutting models, not one page per subsystem. Accounts have none by design: [session isolation](./explanation/session-isolation.md) owns the account-to-group relationship, and [accounts](./reference/accounts.md) opens with the model its contract rests on — a third page could only restate them ([ADR-0012](./decisions/ADR-0012-docs-architecture.md)).

## Reference

- [CLI surface](./reference/cli-surface.md) — the wrapper's own grammar, the flags it claims, and the parser shape that makes verbatim passthrough work.
- [Process runtime](./reference/process-runtime.md) — child resolution, the recursion guard, process groups, the signal matrix, and reaping.
- [Exit codes](./reference/exit-codes.md) — the wrapper's exit-code matrix and the child-status passthrough rule.
- [XDG storage](./reference/xdg-storage.md) — every artifact's base directory, writer, mode, and lifetime.
- [Accounts](./reference/accounts.md) — what an account is, how one is selected, and the contract of every `account` subcommand.
- [Configuration](./reference/configuration.md) — the precedence ladder and the settings-composition model.
- [Logging and output](./reference/logging-and-output.md) — the stream contract, the log schema, colour rules, and the `doctor` check catalog.
- [Coding conventions](./reference/coding-conventions.md) — naming, visibility, error layering, and the panic policy.
- [Dependencies](./reference/dependencies.md) — the reviewed crate set and the rules for adding to it.
- [Testing and quality](./reference/testing-and-quality.md) — test tools, lanes, gates, and which contract each test locks down.
- [Project governance](./reference/project-governance.md) — binding-rule owners, enforcement, and decision-status authority.
- [Release workflow](./reference/release-workflow.md) — exact branch, automation, authentication, packaging, distribution, and recovery contracts.
- [Prior art](./reference/prior-art.md) — comparable projects, what was inspected, and what was borrowed or rejected.
- [Research tracking](./reference/research-tracking.yaml) — the perishable facts these documents rest on, and when to re-check each.
- [Examples](./reference/examples/) — the configuration files a user copies. Most are generated from the config types and must not be hand-edited; [configuration](./reference/configuration.md#generated-examples-and-schema) says which, and why.

## Guides

- [Development workflow](./guides/development-workflow.md) — pick up queued work, add a command, add a dependency, write a test, run the gate.
- [Release and publishing](./guides/releasing.md) — one-time bootstrap, routine release, emergency publish, and recovery procedures.

The zone grows with the specifications, not with the releases: a guide is written whenever a task needs an ordered procedure, and before `0.1.0` its reader is the implementer ([ADR-0036](./decisions/ADR-0036-write-the-specifications-before-the-code.md)). A page missing today is work outstanding rather than a decision — closed by a real guide, never by a placeholder ([ADR-0012](./decisions/ADR-0012-docs-architecture.md)).

## Decisions

Architecture decision records live in [`decisions/`](./decisions/), numbered and never deleted. A decision that stops being true is superseded by a new record; one that is partly changed keeps its status and gains an `Amended by` pointer. Start a new one from [the template](./decisions/template.md).
