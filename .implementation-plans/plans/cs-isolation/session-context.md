# Isolation R3: Session Context Integration

> Plan: cs-isolation | Round: 3 of 3 | Complexity: M | Executor: prex (EF 1.5) | Repo: repository root

## Context

A wrapper session is an **account plus a profile**, and the two selections are independent ([session isolation](../../../docs/explanation/session-isolation.md)). The child will later (in `cs-wrapper-runtime`) receive the profile's composed settings through the native `--settings` flag, while `CLAUDE_CONFIG_DIR` points at the account. This round exposes both resolutions through `AppContext` so later commands request them without recomputation, and surfaces the resolved paths to `doctor`.

There is no session metadata document and no pruning. `session-meta.json` existed only to hold the terminal group's derivation fingerprint and was retired with the group ([ADR-0065](../../../docs/decisions/ADR-0065-retire-the-terminal-group.md)); composed settings entries are immutable and permanent, so nothing sweeps them ([XDG storage § Cleanup and recovery](../../../docs/reference/xdg-storage.md#cleanup-and-recovery)).

## Previous Rounds

This plan round 1: `adapters/fs.rs`, secure-dir service, state-root resolution. Round 2: `AccountId`/`ProfileId` newtypes, the entry-key computation over supplied inputs, and the store's publication rule. Expect both to exist and compile.

## Scope of This Round

- IN scope: `services/account/dir.rs` `account_dir(root, account)` building `accounts/<account>/` securely step by step; lazy resolution of the account and of the validated `ProfileId` in `AppContext` (resolve once, reuse); the `doctor` inspection helper surfacing the resolved account path and the composed-settings store directory.
- OUT of scope: building the child environment and argv, and spawning (`cs-wrapper-runtime`); real accounts (use `default`, replaced by `cs-accounts-auth`); credentials and the merge itself (later plans); **turning a profile name into an entry path**, which needs the profile file and its ordered pieces — loading those is `cs-config-composition` round 1, so that plan wires round 2's `entry_key` to a resolved profile and extends the `doctor` line with the entry path.

## Current State

### Key Files

- `src/services/account/dir.rs` — add `account_dir`.
- `src/context.rs` — add lazy account and profile resolution.

### Existing Patterns

The path layout, modes, and single-writer rule are specified in [`docs/reference/xdg-storage.md`](../../../docs/reference/xdg-storage.md); what `doctor` reports is in [`docs/reference/logging-and-output.md`](../../../docs/reference/logging-and-output.md).

`account_dir(root, account)` joins the account segment and secures **each level** with the round-1 helpers.

Resolution is lazy and independent: a run may select an account without a profile, or a profile without an account. Neither resolution forces the other, and a run with neither is a valid passthrough that launches the child unchanged.

## Implementation Steps

### First Step: Mark this round as started

In this plan's `queue-rounds.yaml`, set this round's (`item: session-context`) `status` to `doing`.

### Step 1: Account dir

In `services/account/dir.rs`, add `account_dir(root, account)` securing each path segment under `accounts/<account>/` with round 1's secure-dir helpers. Use `account = "default"`.

### Step 2: AppContext integration

Add lazy resolution to `AppContext` — account name → root → `account_dir`, and profile name → validated `ProfileId` — so later commands request either without recomputation, and so a run that selects neither pays for neither. The entry path is not resolved here: its key needs the profile file and every piece, which `cs-config-composition` loads.

### Step 3: Diagnostics

Surface the resolved account path and the composed-settings store directory through the `doctor` inspection helper. The entry path joins that report in `cs-config-composition`, once a profile can be resolved to its pieces.

### Final Step: Update the queue

1. In this plan's `queue-rounds.yaml`, set this round's (`item: session-context`) `status` to `done`.
2. All rounds are now done, so in the top-level `.implementation-plans/queue-plans.yaml` set this plan's (`item: cs-isolation`) `status` to `done`. Leave the plan directory in place.

## Acceptance Criteria

- [ ] The account directory path is `accounts/<account>/` under the resolved state root, with each level secured.
- [ ] `AppContext` exposes the resolved account and the validated profile identifier lazily, each computed at most once, and neither forcing the other.
- [ ] `doctor` reports the resolved account path and the composed-settings store directory.
- [ ] No session metadata document is written and no pruning runs.
- [ ] Tests pass (`cargo nextest run`, pre-commit profile).
- [ ] This plan's `queue-rounds.yaml` shows round `session-context` as `done` and the top-level `queue-plans.yaml` shows `cs-isolation` as `done`.
