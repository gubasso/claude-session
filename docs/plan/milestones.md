# Milestones

The single status surface. A reader consults this and nothing else to know where the work stands.

One line per slice: `<id> <slug> — <status> — <appetite>[ — <note>]`. The slug is the slice's directory under [slices](./slices/). Live work comes first, in execution order, so the top line is the next slice to pick up; `## closed` is a ledger and stays ordered by id. A line with a predecessor opens its note with `after <id>[ and <id>]`, and every id named there is closed or listed above. A slice moves to `## closed` when its status becomes terminal and never moves back. The [development workflow](../guides/development-workflow.md#pick-up-current-work) owns the status vocabulary and the pick-up procedure, and the [charter](./charter.md) owns the appetite unit.

## in flight

- 009 release-readiness — shaped — 1 implementation session — after 003 and 011; readies `0.1.0` for a human release decision
- 005 account-authentication — shaped — 2 implementation sessions — after 009; the `0.2.0` login-mode account rung
- 007 comprehensive-doctor — shaped — 1 implementation session — after 005 and 012; proves shared reporting over feature-owned checks
- 008 cli-artifacts — shaped — 1 implementation session — after 005; proves generated artifacts for the account MVP
- 013 token-account-lifecycle — shaped — 2 implementation sessions — after 007 and 008; the `0.3.0` token lifecycle rung
- 004 profile-composition — shaped — 2 implementation sessions — after 013; the `0.4.0` single-piece profile rung
- 014 full-profile-composition — shaped — 2 implementation sessions — after 004; the `0.5.0` full-composition rung
- 006 proxy-guide-and-research — shaped — 1 implementation session — after 003; filler only while the next versioned rung is blocked on measurement

## closed

- 001 native-passthrough-foundation — done — 4 implementation sessions
- 002 secure-session-storage — done — 3 implementation sessions — gave the state namespace its first durable artifacts
- 003 native-child-exec — done — 2 implementation sessions — replaced supervision with the exec
- 010 native-repository-gates — done — 1 implementation session — replaced four shell check-scripts
- 011 entry-point-and-contract-repair — done — 3 implementation sessions — repaired contracts 001 claimed but did not meet
- 012 human-presentation — done — 2 implementation sessions — decided the colour contract every later renderer applies
