# ADR-0109: Discriminate namespaces across kernels

## Context and Problem Statement

A namespace component ([ADR-0107](./ADR-0107-scope-a-terminal-to-its-namespace.md)) names a kernel namespace by fingerprinting its `/proc` link value. Those inode numbers are per-kernel counters with fixed initial values, so a separate kernel reaching this state tree over a filesystem share — a virtual machine mounting the same directories — reports the same link value as the host, and its first pane lands in the host's session directory. That interleaves the very files [ADR-0102](./ADR-0102-key-child-state-by-terminal.md) split.

## Considered Options

- Fold a per-kernel discriminator into the namespace fingerprint's preimage.
- Add a machine-level path component above the namespace directory.
- Detect virtualization and only then change the name.

## Decision Outcome

Chosen option: fold the discriminator into the preimage — the component keeps its grammar, width, and place, and two kernels stop agreeing on it.

The discriminator is the machine identifier, because session directories are durable and must survive reboots, and it is read through a two-rung ladder: the kernel's machine identifier, then its boot identifier, honest at the cost of stranding per boot. Where neither reads, the rung names nothing and the existing refusal absorbs it, exactly as an unreadable namespace already does. The fingerprint covers the discriminator and the link value with an unambiguous separator, so no pair of values can imitate another.

Every existing namespace directory is stranded once, which is [ADR-0102](./ADR-0102-key-child-state-by-terminal.md)'s recorded posture: session directories accumulate, nothing prunes them, and a launch rebuilds what matters. A guest whose machine identifier regenerates per boot strands per boot, and a recreated guest is honestly a new machine.

Detection was rejected because it is a heuristic answering a question the machine identifier answers exactly. The extra path component was rejected because the discriminator has no meaning apart from the namespace it qualifies.

## Consequences

- Good: two kernels sharing one state tree never share a session directory, with no new path shape.
- Bad: one-time stranding of every existing session directory.
- Bad: the component is no longer derivable from the namespace link alone.

## Status

Implemented

Shaped by [032](../plan/slices/032-vm-safe-namespace-identity/README.md). Amends [ADR-0107](./ADR-0107-scope-a-terminal-to-its-namespace.md).
