# ADR-0046: Support Linux and a single child baseline

## Context and Problem Statement

[The process runtime](../reference/process-runtime.md#platform-scope) claimed Unix, and [research tracking](../reference/research-tracking.yaml) carries facts about the child that were only ever measured on one platform against one build. A supported surface wider than the measured one turns every perishable fact into an unstated guess, and [ADR-0044](./ADR-0044-audit-wrapper-spellings-against-the-child-inventory.md)'s inventory audit has no meaning without a named baseline to audit against.

## Considered Options

- Support Linux only, against child `2.1.220` or newer.
- Keep claiming Unix, and mark every unmeasured platform's behaviour unverified.
- Support Unix, and gate each platform on a per-platform inventory fixture.

## Decision Outcome

Chosen option: Linux and `2.1.220` or newer — the supported surface is exactly the surface the project measures, and widening it is a decision with its own evidence rather than an omission.

Linux is the supported target. Other Unix systems are neither claimed nor deliberately broken: the contracts rest on process groups, POSIX signals, controlling terminals, and Unix file modes, so they are expected to hold, but nothing asserts it and no gate proves it. Windows remains out of scope for the reason it always was — the signal and process-group model has no direct equivalent.

`2.1.220` is the child baseline. Every child fact this project records is measured against it or newer, and the inventory fixture is labelled with the version it was taken from. This is a documentation and evidence baseline, not a new pre-spawn gate: the only version check that refuses to launch is still [ADR-0031](./ADR-0031-enforce-the-child-refresh-lock-version-floor.md)'s `login`-mode floor.

## Consequences

- Good: every recorded child fact has a stated platform and version, so a stale one is visible.
- Good: the inventory audit has one fixture to compare against rather than a matrix.
- Bad: a macOS user is unsupported in documentation even where the code works.
- Bad: adding a platform means re-measuring every perishable fact, not just adding a CI target.

## Status

Accepted
