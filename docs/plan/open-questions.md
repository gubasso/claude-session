# Open questions

## Q-002 — Does the child baseline still guarantee cross-process refresh locking?

Blocks: slice 013 login-mode version-floor acceptance.

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
