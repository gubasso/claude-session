# ADR-0090: Require account and profile before child launch

## Context and Problem Statement

A passthrough could reach the child without an account or profile, leaving the session unattributable. The wrapper needs one launch-readiness rule without changing child argv, streams, signals, or status after readiness is established.

## Considered Options

- Require both an account and a profile.
- Require only authentication.
- Retain unbound stock behaviour.

## Decision Outcome

Chosen option: require both axes — every child launch is attributable to one credential and one settings profile.

Only passthrough launch is gated; wrapper verbs remain available without either binding. Selections may come from flags or configuration, including `default_account`, the last-used account marker, and `default_profile`. After both resolve, native passthrough remains unchanged. A missing selection is a pre-exec `Config` failure at exit 78.

No last-used profile marker is added. `default_profile` already expresses durable user intent without new state or ADR-0015's rejected persisted binding.

## Consequences

- Good: every launched child has an accountable credential and settings identity.
- Good: bare `claude-session` still launches when configuration resolves both selections.
- Bad: a previously valid unbound passthrough now requires setup or selection.

## Status

Implemented

Amended by [ADR-0096](./ADR-0096-bind-a-profile-to-an-account.md), which keeps the two-axis requirement while replacing the refusal of persisted profile state with a per-account binding. Supersedes [ADR-0058](./ADR-0058-behave-as-stock-claude-by-default.md). Amends the unbound-launch clauses in [ADR-0031](./ADR-0031-enforce-the-child-refresh-lock-version-floor.md) and [ADR-0050](./ADR-0050-name-the-profile-surface-once.md). Enacted by [the exec sequence](../reference/process-runtime.md#the-exec).
