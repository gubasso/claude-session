# 027 — Superseded credential retirement

## Goal

Stop a mode switch from abandoning a live credential. Recording the new mode is the whole of a switch today, so the artifact the old mode used stays on disk unreachable: no read reaches it, no rotation refreshes it, no surface reports it, and only removing the whole account clears it. A stored token outlives its account's token mode by up to a year that way, and a child credential outlives its login mode until the account is deleted.

## Appetite

1 implementation session.

## Core

A login that changes an account's mode leaves nothing of the mode it replaced, and says so.

## In scope

- Unlinking the artifact the newly recorded mode does not use, inside the commit that records it and under the lock that commit already holds.
- Unconditional retirement, so the same step also clears an account orphaned by a switch made before this slice.
- A guard before the unlink, so a replaced `config` directory cannot aim it outside the account.
- Failing the login when the retirement fails, naming the path that still holds a credential and saying the new one is committed and is not being undone.
- One report line, and one JSON key, present only when something was retired.
- One decision recording that the wrapper unlinks a child credential here, and [accounts](../../../reference/accounts.md) amended where it promises not to touch one.

## Out of scope

- Reading, copying, or fingerprinting either artifact. Retirement is an unlink of a known path, which is the one operation that needs no knowledge of the contents.
- A `doctor` check or an `account status` field for an orphan. The next login on that account clears it, and a check for a state the wrapper repairs on its own discriminates nothing ([ADR-0051](../../../decisions/ADR-0051-let-every-surface-element-discriminate.md)).
- Retiring anything at removal. [Ordered removal](../../../reference/accounts.md#removal) already unlinks both and commits differently.
- Revoking either credential at the provider. The wrapper speaks no endpoint ([ADR-0026](../../../decisions/ADR-0026-store-and-inject-a-long-lived-subscription-token.md)).
- A confirmation prompt. The switch is the instruction, and a login that stops to ask would break the non-interactive path 026 exists for.

## Governed by

- [AGENTS.md](../../../../AGENTS.md)
- [Charter](../../charter.md)
- [ADR-0011](../../../decisions/ADR-0011-isolate-credentials-by-seed-and-session.md)
- [ADR-0025](../../../decisions/ADR-0025-share-one-native-login-per-account.md)
- [ADR-0026](../../../decisions/ADR-0026-store-and-inject-a-long-lived-subscription-token.md)
- [ADR-0030](../../../decisions/ADR-0030-use-account-login-for-wrapper-authentication.md)
- [ADR-0051](../../../decisions/ADR-0051-let-every-surface-element-discriminate.md)
- [ADR-0087](../../../decisions/ADR-0087-keep-the-credential-lock-beside-the-account.md)
- [ADR-0088](../../../decisions/ADR-0088-model-nothing-the-child-already-owns.md)
- [ADR-0093](../../../decisions/ADR-0093-write-every-non-machine-surface-for-a-person.md)
- [ADR-0096](../../../decisions/ADR-0096-bind-a-profile-to-an-account.md)
- [ADR-0099](../../../decisions/ADR-0099-declare-the-plan-a-token-cannot-carry.md)
- [ADR-0100](../../../decisions/ADR-0100-bootstrap-a-saved-login-from-a-refresh-token.md)
- [Accounts](../../../reference/accounts.md)
- [XDG storage](../../../reference/xdg-storage.md)
- [Presentation](../../../reference/presentation.md)
- [Coding conventions](../../../reference/coding-conventions.md)
- [Testing and quality](../../../reference/testing-and-quality.md)

## Acceptance

- When a login records an account as a saved login, the wrapper shall unlink that account's stored token. -> accounts::a_native_login_retires_the_token_it_supersedes
- When the login recording it is the terminal-less one, the wrapper shall retire on the same terms. -> accounts::a_refresh_login_retires_the_token_it_supersedes
- When a login records an account as a token account, the wrapper shall unlink that account's child-owned saved login, leave the directory holding it, and leave the token it recorded. -> accounts::a_token_login_retires_the_saved_login_it_supersedes
- When an account holds neither artifact, the login shall succeed and report no retirement. -> accounts::a_login_that_supersedes_nothing_succeeds_and_says_nothing
- When a login retires an artifact, the report shall say so. -> accounts::a_login_that_retires_a_credential_reports_it
- When the retirement cannot complete, the wrapper shall fail the login, keep the credential it committed, and name the path that still holds one. -> accounts::a_retirement_that_cannot_complete_keeps_the_credential_it_committed
- When another login commits against the same account first, the wrapper shall refuse before recording a mode and retire nothing.

## Rabbit holes

- Retiring before the metadata rename; escape: the rename is the commit, so an interruption before it must leave the old credential working rather than the account holding neither.
- Reading the replaced metadata to decide what to retire; escape: the mode just recorded already says which artifact is live, and the unconditional form is what repairs an account orphaned earlier.
- Undoing the login when the retirement fails; escape: the credential is already durable, and destroying a working credential over a leftover one is the larger harm ([`commit_binding`](../../../../src/commands/account.rs) sets the precedent).
- Reusing the removal module's `unlink`; escape: its diagnostic says the account is already unusable, which is false of an account that just logged in.
- Retiring on the strength of a credential judged before the lock was taken; escape: a login that lost the race would unlink the token the winner just committed while the saved login it thinks it has is already gone, leaving neither. The question is asked again inside the lock, and the last acceptance line has no test id because forcing that interleaving needs a facility the harness does not have (Q-011).
- Treating a leftover as a `doctor` finding as well; escape: two surfaces for a state the next login clears is the duplication [ADR-0051](../../../decisions/ADR-0051-let-every-surface-element-discriminate.md) rejects.

## Done when

A login that changes an account's mode leaves only the artifact the recorded mode uses, an account carrying a leftover from an earlier switch is repaired by its next login, a retirement that cannot proceed safely fails the login without undoing it and names the surviving path, the report distinguishes a retirement from none, the unlink of a child credential is recorded and [accounts](../../../reference/accounts.md) no longer reads as a promise never to, and `just hooks` is green.

## Revisions

- None.
