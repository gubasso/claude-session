# Open questions

## Q-002 — Does the child baseline still guarantee cross-process refresh locking?

Blocks: slice 005 revalidation of the documented minimum, which the login-mode refusal ships against meanwhile because that check fails closed.

Raised: migrated from the perishable child-version and refresh-lock records.

Exit: measurement, revalidate current official release material and run two concurrent processes across refresh without recording credentials.

## Q-003 — Is the published child settings schema complete enough for strict validation?

Blocks: slice 014's optional validation tightening; permissive unknown-key handling remains the shaped default.

Raised: migrated from the schema tracking entry.

Exit: measurement, record whether the current schema both tracks releases and forbids additional properties.

## Q-004 — Does in-TUI login honor the selected account directory in both stored modes?

Blocks: slice 013 TUI-login warning and remediation acceptance.

Raised: migrated from the two unverified account facts.

Exit: measurement, exercise login mode and token mode in isolated scratch accounts and record only non-secret outcomes.

## Q-005 — Which page should retire the stale supervised-runtime description?

Blocks: reconciliation of `docs/explanation/architecture.md` with implemented ADR-0084 and the exec-owned process runtime.

Raised: slice 005 found that the non-governing explanation still describes supervision and post-flight marker writes.

Exit: slice revision, assign the current exec model one explanation owner and remove the stale supervised sequence.

## Q-006 — Where does requested help become a result for the remaining verbs?

Blocks: the requested-help contract in `docs/reference/cli-surface.md#help` for `doctor` and `version`, which promises that `<verb> --help` and `help <verb>` print on standard output and exit `0`.

Raised: slice 005 implemented that contract for the `account` namespace it added, and found that `disable_help_flag` and `disable_help_subcommand` predate the rung, so `doctor --help`, `version --help`, `help doctor`, and `help version` still exit `Usage`. Repairing them here would change two surfaces this slice does not own, and `help doctor` additionally has to compose the child's own help.

Exit: slice revision, assign the remaining verbs' requested-help surfaces one owning slice and make requested help a result at every spelling the CLI surface names.
