# ADR-0065: Retire the terminal group

## Context and Problem Statement

A group existed to own one composed `settings.json`, its provenance sidecar, and its session metadata ([ADR-0062](./ADR-0062-derive-the-group-from-the-controlling-terminal.md)). [ADR-0064](./ADR-0064-key-composed-settings-by-profile-and-input-digest.md) moves settings and provenance onto a profile-and-input key, and `session-meta.json` held nothing but the derivation fingerprint [ADR-0063](./ADR-0063-claim-a-group-by-its-derivation-fingerprint.md) introduced to guard those settings. The five-rung ladder, the host discriminator, and the claim protocol now key nothing.

## Considered Options

- Keep the group as a session-metadata axis.
- Keep `groups/<group>/<profile>/`, so both axes survive.
- Retire the group and every surface whose only consumer it was.

## Decision Outcome

Chosen option: retire it. A surface earns its place by discriminating something ([ADR-0051](./ADR-0051-let-every-surface-element-discriminate.md)); with settings re-keyed, no wrapper behaviour reads a group. Keeping it as a metadata axis would preserve `ttyname_r(3)`, `getsid(2)`, the discriminator, and the claim check in order to write a document whose content this project never specified. `groups/<group>/<profile>/` keeps the same machinery to key an artifact with no terminal-dependent input.

Retired with it: the `--session <id>` flag, `CLAUDE_SESSION_GROUP`, the `session-identity-derives` and `session-group-claim` checks and their remediations, `session-meta.json`, the group directory and its `.settings.lock`, stale-group pruning, and the terminal-identity prior art these rested on. Dropping `--session` returns that spelling to the child, which is the passthrough change this record authorises.

The account is untouched: one shared child `config/`, one saved login, one project history ([ADR-0025](./ADR-0025-share-one-native-login-per-account.md)).

## Consequences

- Good: no terminal, multiplexer, container, or machine-identity input remains anywhere in the storage layout.
- Good: `doctor` loses two checks that could only report a derivation nothing consumes.
- Bad: a run can no longer be given a name; no invocation reads one now, and the profile is the name that matters.
- Bad: `rustix` and `sha2` each lose part of their justification, and each keeps another.

## Status

Accepted

Supersedes [ADR-0062](./ADR-0062-derive-the-group-from-the-controlling-terminal.md) and [ADR-0063](./ADR-0063-claim-a-group-by-its-derivation-fingerprint.md). Their reasoning against multiplexer and emulator variables stays there as history and needs no successor, because nothing derives a terminal identity now. Amends [ADR-0018](./ADR-0018-one-probe-set-with-stable-check-ids.md) and [ADR-0061](./ADR-0061-protect-storage-from-accidental-local-drift.md).
