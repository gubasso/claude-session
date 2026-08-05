# ADR-0063: Claim a group by its derivation fingerprint

## Context and Problem Statement

A terminal name is unique only inside one kernel view. Each mount of `devpts` allocates pty indices independently, so two containers that bind-mount one state tree both derive `pts-0` and silently share a group — merging two sessions' settings, provenance, and metadata with no error. [ADR-0062](./ADR-0062-derive-the-group-from-the-controlling-terminal.md)'s ladder cannot see this: every input it reads is namespace-local.

## Considered Options

- A discriminator prefix on the identifier, off by default.
- Hash the whole derivation, namespaces included, into an opaque identifier.
- Keep a readable identifier and record the full derivation, checked on use.

## Decision Outcome

Chosen option: readable identifier plus a recorded fingerprint, in two parts.

The identifier carries the first bytes of a keyed `/etc/machine-id` hash, falling back to hostname. It is always on: opt-in permits silent collision, and container detection is the tool-specific knowledge ADR-0062 rejects. Kernel boot id is shared by containers.

`session-meta.json` records the rung, terminal identity, session leader and start time, and process and mount namespaces. Absent state is claimed; a match is reused; a mismatch never opens settings and instead derives a suffixed group with a warning.

The fingerprint is what makes the guarantee unconditional, including where the discriminator itself fails — a cloned machine identity in a container image is the case a prefix alone cannot catch. Hashing everything into an opaque identifier would buy the same collision resistance and cost a diagnosable directory name.

## Consequences

- Good: two genuinely separate sessions can never share a group, by check rather than by probability.
- Bad: identifiers gain a fixed prefix that means nothing to a reader.
- Bad: a machine identity is confidential, so it is only ever stored hashed ([machine-id(5)](https://man7.org/linux/man-pages/man5/machine-id.5.html)).

## Status

Superseded

Superseded by [ADR-0065](./ADR-0065-retire-the-terminal-group.md) — with settings keyed by profile and input digest under [ADR-0064](./ADR-0064-key-composed-settings-by-profile-and-input-digest.md), the fingerprint guards nothing and the metadata that carried it describes nothing.
