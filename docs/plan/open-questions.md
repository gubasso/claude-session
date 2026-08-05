# Open questions

## Q-001 — When can slice acceptance name tests?

Blocks: adoption of test-name arrows in every slice Acceptance section; assertion-only execution is not blocked.

Raised: during the documentation refactor while the crate has no test tree.

Exit: slice revision, after slice 001 creates real tests and the same change adds a hook that resolves every named test.

## Q-002 — Does the child baseline still guarantee cross-process refresh locking?

Blocks: slice 005 login-mode version-floor acceptance.

Raised: migrated from the perishable child-version and refresh-lock records.

Exit: measurement, revalidate current official release material and run two concurrent processes across refresh without recording credentials.

## Q-003 — Is the published child settings schema complete enough for strict validation?

Blocks: slice 004's optional validation tightening; permissive unknown-key handling remains the shaped default.

Raised: migrated from the schema tracking entry.

Exit: measurement, record whether the current schema both tracks releases and forbids additional properties.

## Q-004 — Does in-TUI login honor the selected account directory in both stored modes?

Blocks: slice 005 TUI-login warning and remediation acceptance.

Raised: migrated from the two unverified account facts.

Exit: measurement, exercise login mode and token mode in isolated scratch accounts and record only non-secret outcomes.

## Q-005 — Are the external first-release bootstrap prerequisites complete?

Blocks: slice 009 Done when.

Raised: migrated from the release round's external operator prerequisites.

Exit: measurement, record that `develop`, the GitHub App secrets, and both branch rulesets exist before changing slice 009 to done.
