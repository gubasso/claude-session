# Open questions

## Q-002 — Does the child baseline still guarantee cross-process refresh locking?

Blocks: slice 005 revalidation of the documented minimum, which the login-mode refusal ships against meanwhile because that check fails closed.

Raised: migrated from the perishable child-version and refresh-lock records.

Exit: measurement, revalidate current official release material and run two concurrent processes across refresh without recording credentials.

## Q-005 — Which page should retire the stale supervised-runtime description?

Blocks: reconciliation of `docs/explanation/architecture.md` with implemented ADR-0084 and the exec-owned process runtime.

Raised: slice 005 found that the non-governing explanation still describes supervision and post-flight marker writes.

Exit: slice revision, assign the current exec model one explanation owner and remove the stale supervised sequence.

## Q-006 — Where does requested help become a result for the remaining verbs?

Blocks: the requested-help contract in `docs/reference/cli-surface.md#help` for `doctor` and `version`, which promises that `<verb> --help` and `help <verb>` print on standard output and exit `0`.

Raised: slice 005 implemented that contract for the `account` namespace it added, and found that `disable_help_flag` and `disable_help_subcommand` predate the rung, so `doctor --help`, `version --help`, `help doctor`, and `help version` still exit `Usage`. Repairing them here would change two surfaces this slice does not own, and `help doctor` additionally has to compose the child's own help.

Exit: slice revision, assign the remaining verbs' requested-help surfaces one owning slice and make requested help a result at every spelling the CLI surface names.

## Q-007 — Is a composed entry named by its inputs, or verified against them?

Blocks: whether `docs/reference/xdg-storage.md#composed-settings-entries` keeps reuse as a digest comparison or promotes it to a content check.

Raised: the slice 014 review observed that reuse turns solely on the sidecar's recorded digest matching the one the inputs recompute, and that the digest never ranges over the settings file's own bytes. A sidecar reduced to a correct `digest` field alone is therefore accepted, and a corrupted settings member is reused unread. The implementation matches its owner page verbatim, and the page claims the check makes "two profiles never share settings" a check rather than a probability — it never claims tamper resistance.

Exit: ADR, decide whether the entry invariant is "named by its inputs" or "verified against its inputs", then amend the owner page and the reuse path together. The cost of the second reading is a byte comparison on every launch; the case it defends against presupposes the user corrupting their own mode `0600`, guard-validated state.
