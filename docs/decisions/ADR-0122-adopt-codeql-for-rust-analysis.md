# ADR-0122: Adopt CodeQL for Rust analysis

## Context and Problem Statement

The Scorecard SAST zero is misleading rather than false: this project runs Clippy at `all`, `pedantic`, and `nursery`, forbids `unsafe_code`, and runs `cargo-audit` and `cargo-deny`. Scorecard recognizes tool identities, so raising its number is not a reason to add a scanner.

The reason is this wrapper's process-launching and XDG-path-resolving surface. On 2026-09-09, the CodeQL Rust corpus held 20 security queries across 17 CWE directories; CWE-078 command injection, CWE-022 path traversal, CWE-117 log injection, and CWE-798 hard-coded credentials reach it directly. [Rust support became generally available](https://github.blog/changelog/2025-10-14-codeql-scanning-rust-and-c-c-without-builds-is-now-generally-available/) on 2025-10-14 and requires build-free analysis.

## Considered Options

- Run CodeQL beside the Scorecard workflow and gate nothing.
- Put CodeQL in `ci.yml` and join it to the required `gate` job.
- Run Semgrep Community Edition.

## Decision Outcome

Chosen option: run CodeQL beside Scorecard, because its queries cover security-sensitive surfaces that one reviewer cannot exhaustively check by hand.

`.github/workflows/codeql.yml` analyzes Rust with `build-mode: none` on pull requests, pushes to `master`, and weekly, reporting into the security tab. It gates nothing. One maintainer reviews everything, so a false finding holding a merge costs more than a true finding waiting there; [ADR-0120](./ADR-0120-adopt-the-openssf-scorecard-workflow.md) established this placement.

Putting the job in `ci.yml` is the only option that stops a defect from landing and would suit more reviewers. Semgrep's smaller Rust corpus adds less, and its unsafe-use rule is already unreachable.

## Consequences

- Good: process, path, log, and credential risks receive a specialized independent reading.
- Bad: three mutable action references and a scheduled workflow require maintenance.
- Bad: Scorecard may rise because it recognizes CodeQL, but that change measures tool identity rather than the reason for adoption.
- Constraint: [the terms](https://github.com/github/codeql-cli-binaries/blob/main/LICENSE.md) permit use while this public repository declares the OSI-approved `MIT OR Apache-2.0` licence. If either visibility or licence changes, this adoption stops being permitted without a paid GitHub Code Security seat.
- Observation: GitHub detects the pointer `LICENSE` as `Other`; the declared SPDX expression is authoritative.

## Status

Implemented

Enacted in [the CodeQL workflow](../../.github/workflows/codeql.yml).
