# Open questions

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

## Q-006 — Does the wrapper colour anything, and should it?

Blocks: nothing. The colour ladder is specified, implemented as a pure function, tested, and its result discarded — so the test and the ladder both pass against a program with no colour output at all.

Raised: slice 011, which found the computed value dropped at the context constructor and left both the code and the specification alone rather than deciding by deletion.

Exit: ADR, either colour a named surface and assert the escape bytes, or remove the ladder from the code and the reference together.
