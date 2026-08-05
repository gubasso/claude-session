# Native passthrough foundation

## Goal

Replace the placeholder with a working stock-compatible pass-through.

## Appetite

4 implementation sessions.

## Core

Unknown native argv is byte-preserved, the child runs, and its status is returned.

## In scope

- Error, logging, UI, context, and configuration plumbing.
- The complete wrapper-owned parser and verb stubs.
- Version reporting for the wrapper and resolved child.
- Boundary, output, documentation, and quality gates.

## Out of scope

- Secure account/profile storage, robust signal supervision, authentication, and settings composition.
- Parsing or modelling the child's grammar.

## Governed by

- [AGENTS.md](../../../../AGENTS.md)
- [Architecture](../../../explanation/architecture.md)
- [Wrapper model](../../../explanation/wrapper-model.md)
- [CLI surface](../../../reference/cli-surface.md)
- [Process runtime](../../../reference/process-runtime.md)
- [Exit codes](../../../reference/exit-codes.md)
- [Logging and output](../../../reference/logging-and-output.md)
- [Configuration](../../../reference/configuration.md)
- [XDG storage](../../../reference/xdg-storage.md)
- [Coding conventions](../../../reference/coding-conventions.md)
- [Dependencies](../../../reference/dependencies.md)
- [Testing and quality](../../../reference/testing-and-quality.md)

## Acceptance

- When unknown native arguments are supplied, the wrapper shall forward their bytes, order, count, empty values, and separator unchanged.
- When the child exits or is terminated, the wrapper shall return the same observable status.
- If the resolved child is missing or invalid, then the wrapper shall return the documented typed error without recursing.
- While wrapper-owned commands remain stubs, the wrapper shall keep native passthrough usable.

## Rabbit holes

- Full process supervision; escape: keep the minimal spawn seam replaceable for slice 003.
- Feature-complete verb handlers; escape: expose only the documented stub surface.

## Done when

The targeted `cargo nextest` and `assert_cmd` passthrough checks pass, followed by the repository documentation and quality hooks.

## Revisions

None.
