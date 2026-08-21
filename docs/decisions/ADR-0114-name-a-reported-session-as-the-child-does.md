# ADR-0114: Name a reported session as the child does

## Context and Problem Statement

`session list` names every row by the directory the wrapper made, `agent-<pid>-<hex>`. The person who has to act on a row knows their sessions by the child's name for them — the one in a status line, the one `/rename` sets — and no wrapper-owned state holds it. A report whose subject a reader cannot recognise is one they cannot act on.

## Considered Options

- Keep naming a row by its directory.
- Let the wrapper invent and store a name of its own.
- Read the child's peer registration, verified against the wrapper's own witness.

## Decision Outcome

Chosen option: read the registration. The directory name is an internal identifier, which [ADR-0093](./ADR-0093-write-every-non-machine-surface-for-a-person.md) puts in `--json`; a wrapper-invented name would be a second name for one thing, while the child's is already on the reader's screen.

The join is the registration's process identifier and start time against the witness the launch wrote, so a reissued identifier cannot lend its name to another agent's directory. A registration that does not verify, and a name holding a control character, name nothing: the row keeps its directory name.

[ADR-0089](./ADR-0089-carry-a-child-owned-fact-only-against-an-obligation.md)'s three obligations do not reach a name read back out of the child. This adds a fourth, `name-the-subject`: the child's name for a thing a wrapper report is about, carried so the report names its subject as its reader does. It authorises the name and nothing else in the record.

## Consequences

- Good: a row names what the reader sees everywhere else.
- Good: the carry fails safe — a changed record costs a name, never a wrong one.
- Bad: the record is undocumented, so its freshness becomes tracked work.
- Bad: a fourth obligation widens what the registry admits, and the record's own `name` key is an ordinary word the discovery scan cannot see, which widens [Q-012](../plan/open-questions.md#q-012--which-obligation-authorizes-carrying-the-childs-credential-filename).

## Status

Implemented

Amends [ADR-0089](./ADR-0089-carry-a-child-owned-fact-only-against-an-obligation.md) with a fourth obligation; the three it names are unchanged. Enacted by [`domain::registration`](../../src/domain/registration.rs) and the report reading it. Shaped by [037](../plan/slices/037-session-report-by-name/README.md).
