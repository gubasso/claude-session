# ADR-0066: Ship one Linux artifact

## Context and Problem Statement

[ADR-0038](./ADR-0038-distribute-binaries-with-cargo-dist.md) configured cargo-dist for four architectures and two installers, chosen before [ADR-0046](./ADR-0046-support-linux-and-a-single-child-baseline.md) fixed the supported surface at Linux. Two of those artifacts are macOS builds of a program the documentation says is unsupported there, and one installer is PowerShell, which the supported platform does not run. A published binary is a support claim: whoever downloads it expects the project to care when it breaks, and nothing here measures, tests, or runs on the platforms it was being published for.

## Considered Options

- Keep four targets and treat the extras as best-effort.
- Keep both Linux architectures and drop macOS.
- Build only `x86_64-unknown-linux-gnu`, with a shell installer.

## Decision Outcome

Chosen option: one target, `x86_64-unknown-linux-gnu`, and the shell installer alone. A published artifact and a supported platform are the same claim, so the artifact list is the platform list. `aarch64-unknown-linux-gnu` goes with the rest: it is not the architecture this project is developed or verified on, so shipping it would restate the problem one architecture smaller. Adding a target back is a configuration line and a new record, which is the right cost for making a new promise.

No release has been tagged, so no consumer loses an artifact.

The generated `.github/workflows/release.yml` builds its matrix from `dist plan` at run time and names no target, so this changes `dist-workspace.toml` only.

## Consequences

- Good: every published artifact is on the platform the specifications, the child baseline, and the perishable facts were measured against.
- Good: the installer set stops offering a shell the supported platform does not have.
- Bad: an ARM Linux or macOS user must build from source, with no artifact and no stated intent to provide one.

## Status

Accepted

Amends [ADR-0038](./ADR-0038-distribute-binaries-with-cargo-dist.md), which keeps cargo-dist as the generator and loses its target and installer list.
