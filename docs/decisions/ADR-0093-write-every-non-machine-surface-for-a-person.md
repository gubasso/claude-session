# ADR-0093: Write every non-machine surface for a person

## Context and Problem Statement

The presentation contract says how a human surface looks but not who it is written for, so `doctor` grew a report that leads with kebab check ids, ends in `wrapper status=warn total=16 passed=12`, and prints skip reasons that never say what would make a check apply. Each line is defensible on its own and the whole is unreadable to the person it is for. Every later verb will re-decide this alone unless the audience is a rule.

## Considered Options

- Make the human format a person's format by rule, and let `--json` own every identifier, count, and code.
- Keep one wording for both audiences, tuned until it serves neither badly.
- Leave the audience to each verb, and review wording case by case.

## Decision Outcome

Chosen option: make the human format a person's format — a per-verb `--json` already exists ([ADR-0024](./ADR-0024-machine-output-is-a-per-verb-flag.md)) and already carries every field the human report would stop printing, so the split costs a caller nothing and ends the compromise.

## Consequences

- Good: a reader gets a consequence and a next action where one exists, and never has to decode an identifier to learn what is wrong.
- Good: the next renderer inherits the audience instead of re-deriving it.
- Bad: a script parsing human `doctor` output breaks, and must move to `--json`.
- Bad: every human string now carries an editorial obligation, and a hasty one reads worse than the terse line it replaced.

## Status

Implemented

The rule is rule 7 of [the presentation contract](../reference/presentation.md#the-contract); [doctor](../reference/doctor.md#the-report) is the first surface written under it. It binds the error diagnostic and the log mirror too, which meet it when their renderers are next touched. [Slice 020](../plan/slices/020-human-first-diagnostics/README.md) enacts it. The identifiers it removes from human output stay public in `--json` and `--list`, so [ADR-0018](./ADR-0018-one-probe-set-with-stable-check-ids.md) is unaffected.
