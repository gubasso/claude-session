# Accounts & Auth R3: Launch Authentication, Precedence & the Version Floor

> Plan: cs-accounts-auth | Round: 3 of 4 | Complexity: L | Executor: prex (EF 1.5) | Generated: 2026-06-19 | Repo: repository root

## Context

Before spawning the child, `claude-session` resolves the selected account's **stored** mode and contributes the authentication variables that mode calls for — nothing more. It also detects the ambient child mechanisms that outrank a subscription account, warns about them without stripping them, and refuses a login-mode launch below the child version floor. This round wires all of that into the runtime's environment construction and adds the post-flight last-used marker update. Round 1 built the store and resolver; round 2 wrote the modes this round reads.

## Previous Rounds

This plan R1: account store, `auth-mode.json`, last-used marker, resolver. R2: `account login` in both modes. `cs-isolation`: resolved account directory and composed-settings entry. `cs-wrapper-runtime`: child resolution, environment construction, spawn-and-wait, post-flight. Expect all to exist.

## Scope of This Round

- IN scope: `services/account/launch.rs` — resolve the selected account and its stored mode, and return the wrapper's contribution to the child environment (`CLAUDE_CONFIG_DIR` for the account, plus the retrieved token in token mode); token retrieval performed **immediately before spawn**; ambient higher-precedence detection and its standard-error warning; the token-over-login shadowing report; the login-mode child version floor check that fails **before** spawn; the post-flight last-used marker update; wiring all of it into the runtime's environment construction and post-flight steps.
- OUT of scope: the `account` CLI verbs and their reports (round 4); composing settings (`cs-config-composition`); quota-aware selection, which no decision record authorizes.

## Current State

### Key Files

- `src/services/account/launch.rs` — new.
- `src/commands/pass_through.rs` — call the launch resolution while building the child environment.
- `src/services/account/store.rs` — read `auth-mode.json`, the token, and the marker.

### Existing Patterns

The mode-to-environment table, the ambient mechanisms that outrank a selected account, and the warning surfaces are specified in `docs/reference/accounts.md`. The ordered spawn sequence this round hooks into — including token retrieval immediately before spawn and the post-flight obligations — is in `docs/reference/process-runtime.md`. The version floor and its failure are `docs/decisions/ADR-0031-enforce-the-child-refresh-lock-version-floor.md`.

Three rules govern the design, and each is a rule about what **not** to do.

**The wrapper contributes variables; it never removes them.** Every inherited variable survives, and ambient authentication is never stripped. A higher-precedence ambient mechanism produces a warning on standard error and nothing else — silently defeating a credential the user deliberately exported is worse than running under it.

**The stored mode decides, not the ambient state.** Mode is chosen once by `account login` and resolved deterministically on every later run. Detection informs the warning; it never rewrites the mode.

**The floor is scoped to what depends on it.** A below-floor child hard-fails a `login`-mode launch, because shared-login correctness rests on the child's cross-process refresh coordination. Token mode and a passthrough with no selected account are never blocked by it, and `doctor` reports the same condition as a warning rather than a failure — the same fact, two severities, because only one of them is a precondition.

A passthrough with no selected account receives neither wrapper authentication variable.

## Implementation Steps

### First Step: Mark this round as started

In this plan's `queue-rounds.yaml`, set this round's (`item: launch-auth-and-precedence`) `status` to `doing`.

### Step 1: Mode resolution and environment contribution

In `services/account/launch.rs`, resolve the selected account through round 1's resolver, read its stored mode, and return the variables `docs/reference/accounts.md` credits to that mode. Retrieve and validate a token immediately before spawn, never earlier.

### Step 2: Precedence detection and warnings

Detect the ambient mechanisms that outrank the selected account and emit the standard-error warning. Report token-over-login shadowing the same way. Preserve every inherited variable.

### Step 3: Version floor gate

Before spawn, in `login` mode only, compare the resolved child version against the floor and fail with the detected version, the requirement, and the upgrade. An unparsable version fails the same way.

### Step 4: Post-flight and wiring

Update the last-used marker in the runtime's post-flight step. Wire the launch resolution into the child environment construction in `pass_through.rs`. A post-flight failure is reported and **must not change the child's exit status** — see `docs/reference/process-runtime.md`.

### Step 5: Tests

Test each mode's environment contribution against a stubbed child that prints its own environment; test that ambient variables survive and produce a warning; test that a below-floor child blocks a login-mode launch and does not block token mode or an unselected passthrough.

### Final Step: Update the queue

1. In this plan's `queue-rounds.yaml`, set this round's (`item: launch-auth-and-precedence`) `status` to `done`.

## Acceptance Criteria

- [ ] Each stored mode contributes exactly the variables `docs/reference/accounts.md` credits it, and no others.
- [ ] No inherited variable is removed, and ambient authentication produces a warning rather than a change.
- [ ] A token is retrieved immediately before spawn, never cached across the run.
- [ ] A below-floor or unparsable child version fails a `login`-mode launch before spawn, and blocks neither token mode nor an unselected passthrough.
- [ ] The last-used marker updates post-flight without changing the child's exit status.
- [ ] This plan's `queue-rounds.yaml` shows round `launch-auth-and-precedence` as `done`.

## Next Round

Round 4 (`account-commands`) exposes the `account` verb tree with machine output and redaction, and adds the two account checks to the `doctor` catalog.
