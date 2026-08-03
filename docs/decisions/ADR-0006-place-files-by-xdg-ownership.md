# ADR-0006: Place files by XDG ownership, with sessions in state

## Context and Problem Statement

`claude-session` writes credentials, per-session configuration directories, generated settings, metadata, and logs. XDG compliance is a standing contract, but it only names the base directories — not which artifact goes where. Getting this wrong is expensive to reverse, because credentials end up in the answer.

## Considered Options

- One tree under a dot-directory in the home directory, as many tools do.
- XDG bases, with session directories under data.
- XDG bases, with session directories under state and a strict per-artifact ownership rule.

## Decision Outcome

Chosen option: **XDG bases with sessions under state** — a session directory holds durable, machine-specific, non-recreatable state, which is what the state base is for.

Classification is by **who writes it and what losing it costs**. Configuration is user-authored and read-only at runtime, so program-written state never lives there. Cache is by definition safe to delete, so a credential never lives there — a cache-clearing tool that silently logs the user out is a bug presenting as a mystery. Runtime is ephemeral coordination only.

One prohibition is load-bearing: the runtime base has **no portable default** and is genuinely absent in containers and under `cron`. The wrapper degrades explicitly rather than falling back, and durable state **never** relocates into runtime or a shared temporary directory. Every artifact has exactly one writer. See [XDG storage](../reference/xdg-storage.md).

## Consequences

- Good: the home directory stays clean, and each base can be backed up, synced, or cleared according to what it means.
- Good: "reset my configuration" and "clear my cache" have unambiguous answers that cannot destroy credentials.
- Good: the single-writer rule makes concurrency tractable.
- Bad: paths are longer and less guessable than one dot-directory, so `doctor` must report resolved paths.
- Bad: refusing a runtime fallback means some coordination is simply unavailable in a container.
- Bad: mode and ownership checks run on every invocation, not only at creation, which is work on a hot path.

## Status

Accepted

Amended by [ADR-0050](./ADR-0050-name-the-profile-surface-once.md) — the profile artifact moves from `manifests/` to `profiles/`; the ownership rule placing it is unchanged.

Amended by [ADR-0060](./ADR-0060-lock-the-writes-that-are-not-derivable.md) — the runtime base has no artifact, since a lock lives beside the file it guards, so its absence is no longer a degradation to report; the ownership rule is unchanged.

Amended by [ADR-0061](./ADR-0061-protect-storage-from-accidental-local-drift.md) — the mode and ownership checks this record names gain a recorded scope, so what they cost is measured against what they defend; the ownership rule is unchanged.

Amended by [ADR-0064](./ADR-0064-key-composed-settings-by-profile-and-input-digest.md) — composed settings move to a top-level input-addressed `composed/` store; the ownership rule is unchanged.
