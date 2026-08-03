# Isolation R2: Identifiers and the Composed-Settings Store

> Plan: cs-isolation | Round: 2 of 3 | Complexity: L | Executor: prex (EF 1.5) | Repo: repository root

## Context

`claude-session` keys the child's composed settings by **what they were composed from**, never by the terminal that asked for them. Two profiles launched from one pane must never meet, and two runs of one profile must reuse one entry ([ADR-0064](../../../docs/decisions/ADR-0064-key-composed-settings-by-profile-and-input-digest.md)). The terminal group, its derivation ladder, and its claim protocol were retired ([ADR-0065](../../../docs/decisions/ADR-0065-retire-the-terminal-group.md)); nothing in this round reads a terminal, a machine identity, a working directory, or a project.

This round builds the validated `AccountId` and `ProfileId` newtypes, the entry-key computation, and the write-once publication rule. Round 1 produced the secure filesystem primitives and state-root resolution.

## Previous Rounds

`cs-foundation`: crate tree, plumbing, `GlobalArgs` reserving `--account`. This plan round 1: `adapters/fs.rs`, secure-dir service, state-root resolution. Expect those to exist.

## Scope of This Round

- IN scope: `domain/ids.rs` (`AccountId` and `ProfileId` newtypes; validation per [XDG storage § Identifiers](../../../docs/reference/xdg-storage.md#identifiers) — lowercase-ASCII or digit start, `[a-z0-9_-]` body, at most 32 bytes, returning `DomainError`); `services/settings/entry_key.rs` computing the input digest and the entry filenames; `services/settings/store.rs` implementing the exists-verify-or-write rule.
- OUT of scope: the merge itself and the provenance document's contents (`cs-config-composition`); `AppContext` wiring (round 3); account discovery and auth (`cs-accounts-auth`); spawning and argv construction (`cs-wrapper-runtime`).

## Current State

### Key Files

- `src/domain.rs` (+ `src/domain/`) — add `ids.rs`.
- `src/services/settings/` — add `entry_key.rs` and `store.rs`; consume round 1's secure-dir helpers.

### Existing Patterns

The entry grammar, the digest preimage, and the write rule are specified in [`docs/reference/xdg-storage.md` § Composed settings entries](../../../docs/reference/xdg-storage.md#composed-settings-entries); the model is in [`docs/explanation/session-isolation.md`](../../../docs/explanation/session-isolation.md); the generation flow is in [`docs/reference/configuration.md`](../../../docs/reference/configuration.md).

The store lives at `composed/` directly under the state base — a **sibling** of `accounts/`, not a child. No composition input is account-scoped, so an entry is shared by every account that resolves the same profile and pieces.

A user-supplied identifier that fails validation is **rejected, never truncated**, and exits `Usage`. Truncation invites collision, which here means two profiles sharing one settings document.

Hash with `sha2`, already a declared [dependency](../../../docs/reference/dependencies.md). The preimage is versioned and length-prefixed so no field value can imitate a field boundary, and paths are hashed as **raw OS bytes** rather than as UTF-8 strings.

## Implementation Steps

### First Step: Mark this round as started

In this plan's `queue-rounds.yaml`, set this round's (`item: composed-settings-store`) `status` to `doing`.

### Step 1: Newtypes

In `domain/ids.rs`, define validated `AccountId` and `ProfileId` (constructor validation → `DomainError` on violation). Provide `as_str()` and `FromStr`.

### Step 2: Entry key

In `services/settings/entry_key.rs`, compute the SHA-256 input digest over the specified preimage and render the pair of filenames, `profile-<name>-<12 hex>.json` and `profile-<name>-<12 hex>.compose.json`. Return the full digest alongside the truncated one — the sidecar records the full value.

### Step 3: Publication

In `services/settings/store.rs`, implement the rule: if the settings path exists and the sidecar's recorded digest matches the recomputed one, reuse it and compose nothing; if it disagrees, exit `DataFormat` without opening or overwriting; if exactly one member of the pair exists, treat the entry as unverifiable and write both from this run's inputs rather than adopting the survivor; otherwise write both through round 1's atomic sequence at mode `0600`. Take no lock.

### Step 4: Tests

Unit-test identifier validation (accept `work`, `review-2`; reject empty, `Work`, `work/x`, over-length) and the six rejecting behaviours in [`testing-and-quality.md`](../../../docs/reference/testing-and-quality.md): profile isolation, entry key determinism, terminal independence, entry immutability, sidecar mismatch refusal, and partial pair recovery.

### Final Step: Update the queue

1. In this plan's `queue-rounds.yaml`, set this round's (`item: composed-settings-store`) `status` to `done`.

## Acceptance Criteria

- [ ] `AccountId`/`ProfileId` reject invalid identifiers and accept the canonical forms.
- [ ] The same profile and pieces name one entry; changing any piece's content, resolved path, order, or the strategy table names a different one.
- [ ] Terminal state, working directory, and selected account do not enter the name (test-verified).
- [ ] An existing matching entry is reused and never rewritten; a mismatched one is refused without overwrite.
- [ ] No lock is taken on any entry.
- [ ] This plan's `queue-rounds.yaml` shows round `composed-settings-store` as `done`.

## Next Round

Round 3 (`session-context`) wires the resolved account and the validated profile identifier into `AppContext` and surfaces the account path and the store directory to `doctor`. Resolving a profile **name** to an entry path needs the profile file and its pieces, so it lands in `cs-config-composition`.
