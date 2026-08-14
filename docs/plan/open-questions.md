# Open questions

## Q-002 — Does the child baseline still guarantee cross-process refresh locking?

Blocks: slice 005 revalidation of the documented minimum, which the login-mode refusal ships against meanwhile because that check fails closed, and the 2026-08-12 dates on the process-runtime and xdg-storage entries, which record the lock and the isolated-directory credential placement as unproven.

Raised: migrated from the perishable child-version and refresh-lock records.

Exit: measurement, revalidate current official release material and run two concurrent processes across refresh without recording credentials. The 2026-08-12 pass did not: renewal fires on the child's schedule, and published fixes above the baseline show that behaviour still moving, so the window has to be observed rather than forced.

## Q-005 — Which page should retire the stale supervised-runtime description?

Blocks: reconciliation of `docs/explanation/architecture.md` with implemented ADR-0084 and the exec-owned process runtime.

Raised: slice 005 found that the non-governing explanation still describes supervision and post-flight marker writes.

Exit: slice revision, assign the current exec model one explanation owner and remove the stale supervised sequence.

## Q-007 — Is a composed entry named by its inputs, or verified against them?

Blocks: whether `docs/reference/xdg-storage.md#composed-settings-entries` keeps reuse as a digest comparison or promotes it to a content check.

Raised: the slice 014 review observed that reuse turns solely on the sidecar's recorded digest matching the one the inputs recompute, and that the digest never ranges over the settings file's own bytes. A sidecar reduced to a correct `digest` field alone is therefore accepted, and a corrupted settings member is reused unread. The implementation matches its owner page verbatim, and the page claims the check makes "two profiles never share settings" a check rather than a probability — it never claims tamper resistance.

Exit: ADR, decide whether the entry invariant is "named by its inputs" or "verified against its inputs", then amend the owner page and the reuse path together. The cost of the second reading is a byte comparison on every launch; the case it defends against presupposes the user corrupting their own mode `0600`, guard-validated state.

## Q-008 — Should an account report distinguish the durable binding from the effective selection as separate fields?

Blocks: whether `account status --json` keeps pairing `profile`, the account's binding, with `profile_source`, the layer that supplied the profile this run resolved.

Raised: the slice 023 review observed that a project file or a flag makes the two describe different profiles, and that the human form now says so in words while the document leaves a reader to infer it. The pairing satisfies slice 022's acceptance, which asks for the bound profile and the provenance of the profile in force, so this is a clarity question rather than a defect. A companion asymmetry is that `account list --json` carries the binding with no provenance at all.

Exit: ADR, decide whether the two facts are one field pair or two, then move the field set and both renderers together. It changes a published document shape, so it needs a slice rather than an edit.

## Q-009 — What excludes a running child from the one key a login writes?

Blocks: whether [ADR-0098](../decisions/ADR-0098-seed-the-one-child-key-a-launch-cannot-reach.md) can promise that the login's read-modify-write of the child's `.claude.json` preserves the keys beside it, which [accounts](../reference/accounts.md#what-a-login-leaves-ready) states as a fact.

Raised: the slice 024 review observed that the credential lock excludes wrapper writers only. A `claude` already running under the account holds nothing, so a trust record it writes between the login's read and its rename is lost. [ADR-0084](../decisions/ADR-0084-exec-the-child-instead-of-supervising-it.md) leaves no wrapper process alive to hold a lock for the child's lifetime, so the exclusion the wrapper would need does not exist to be taken. The window is one read-modify-write wide, and two concurrent children already race each other over the same file, so this is a limit inherited from sharing one configuration directory rather than one this slice introduced.

Exit: measurement, observe whether the child rewrites `.claude.json` often enough for the window to be reachable in practice. If it is, the decision that follows is between refusing the write while a child of that account is live and accepting the loss as the cost of a shared directory, and either one changes what the reference pages may promise.

## Q-010 — Should a saved-login commit witness this run's exchange, or only a credential's presence?

Blocks: whether [accounts](../reference/accounts.md#refresh-token-bootstrap) may promise that a successful refresh-token login exchanged anything, rather than that a credential is there afterwards.

Raised: the slice 026 review observed that `credentials_committable` answers "a safe credential exists" and carries no notion of the operation asking. Against a fresh account that is exact. Against an account that already holds a saved login it is not: a child exiting successfully without exchanging would commit `login` mode on the strength of a file that predates the run, and for a token account that also replaces the stored mode while leaving the stored token in place. The gate is shared with the native login, where the same limit has always applied and where a person watched the browser flow that produced the credential; the refresh bootstrap is the first path meant to run unattended, which is what makes the limit worth naming. Nothing in child 2.1.220 reaches the bad state — its exchange branch either completes or exits non-zero — so this is a gap in what the gate can prove rather than an observed failure.

Exit: ADR, decide whether the witness becomes operation-specific and whether that changes the native login too. The comparison that would settle it reads a child-owned credential, which [accounts](../reference/accounts.md#what-an-account-is) forbids and only a `stat` escapes, so the decision is between a weaker witness the wrapper may take, an amendment to that prohibition, and accepting the presence test with the limit documented. Whichever wins needs the regression the review named: an account that starts with a credential, a child that exits successfully without touching it, and an assertion about what the mode becomes.
