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
