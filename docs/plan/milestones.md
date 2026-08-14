# Milestones

The single status surface. A reader consults this and nothing else to know where the work stands.

One line per slice: `<id> <slug> — <status> — <appetite>[ — <note>]`. The slug is the slice's directory under [slices](./slices/). Live work comes first, in execution order, so the top line is the next slice to pick up; `## closed` is a ledger and stays ordered by id. A line with a predecessor opens its note with `after <id>[ and <id>]`, and every id named there is closed or listed above. A slice moves to `## closed` when its status becomes terminal and never moves back. The [development workflow](../guides/development-workflow.md#pick-up-current-work) owns the status vocabulary and the pick-up procedure, and the [charter](./charter.md) owns the appetite unit.

## in flight

- 019 original-namespace-restoration — shaped — 1 implementation session — after 018; waits on the predecessor's deprecation, which is the entry condition rather than the work

## closed

- 001 native-passthrough-foundation — done — 4 implementation sessions
- 002 secure-session-storage — done — 3 implementation sessions — gave the state namespace its first durable artifacts
- 003 native-child-exec — done — 2 implementation sessions — replaced supervision with the exec
- 004 profile-composition — done — 2 implementation sessions — after 013; the `0.4.0` single-piece profile rung inside the combined 004 and 014 effort
- 005 account-authentication — done — 2 implementation sessions — after 007; the `0.2.0` login-mode account rung; cut the bespoke Rust PTY harness in favor of the pinned devShell allocator
- 006 external-fact-revalidation — done — 1 implementation session — after 015 and 003; revalidated the two entries nearest their cadence and left the refresh lock and the isolated-directory login unproven under Q-002
- 007 comprehensive-doctor — done — 2 implementation sessions — after 012; the shared report engine every feature rung appends to
- 008 cli-artifacts — done — 1 implementation session — after 005; deferred the man-page output directory to ADR-0086 rather than ship a form no packager needed
- 009 release-readiness — done — 1 implementation session — readied the `0.1.0` candidate; the forge bootstrap stays the operator's
- 010 native-repository-gates — done — 1 implementation session — replaced four shell check-scripts
- 011 entry-point-and-contract-repair — done — 3 implementation sessions — repaired contracts 001 claimed but did not meet
- 012 human-presentation — done — 2 implementation sessions — decided the colour contract every later renderer applies
- 013 token-account-lifecycle — done — 2 implementation sessions — after 007 and 008; the `0.3.0` token lifecycle rung; closed Q-004 from published child behaviour rather than a local measurement
- 014 full-profile-composition — done — 2 implementation sessions — after 004; the `0.5.0` full-composition rung completing the combined 004 and 014 effort
- 015 child-fact-delegation — done — 2 implementation sessions — after 014; made the rule ADR-0088 implied checkable, and left one contested carry to 016
- 016 ambient-credential-scope — done — 1 implementation session — after 015; settled the ambient carry by amending ADR-0089 rather than opening a record beside it
- 017 mandatory-session-binding — done — 2 implementation sessions — after 016; makes a bound account and profile the precondition for an exec
- 018 legacy-coexistence-namespace — done — 2 implementation sessions — after 017; frees every name the shell predecessor claims so both installs can sit on one machine
- 020 human-first-diagnostics — done — 2 implementation sessions — after 018; landed in one session, and fixed the version reading the report was lying about
- 021 requested-help-completion — done — 1 implementation session — after 005; makes requested help a result at the two spellings that rung left open
- 022 account-profile-binding — done — 2 implementation sessions — after 017; landed in one session, and retired three hints naming verbs that never existed
- 023 human-first-verbs — done — 3 implementation sessions — after 020 and 022; a sweep of every wrapper write widened it past the verbs, and split the log mirror from the log file
- 024 launch-ready-after-login — done — 2 implementation sessions — after 005 and 013; landed in one session, and the walk that found it also proved the fix against the real child
- 025 declared-subscription-plan — done — 2 implementation sessions — after 013 and 024; landed in one session, and review caught an ambient value answering for an account that declared none
- 026 refresh-token-bootstrap — done — 2 implementation sessions — after 005 and 025; landed in one session, and proved the child takes the exchange branch by watching it refuse a deliberately invalid grant
- 027 superseded-credential-retirement — done — 1 implementation session — after 013 and 026; landed in one session, and the failure branch turned out reachable without an unwritable parent
