# Milestones

The single status surface. A reader consults this and nothing else to know where the work stands.

One line per slice, ordered by id inside its section: `<id> <slug> — <status> — <appetite>[ — <note>]`. The slug is the slice's directory under [slices](./slices/). Live work comes first; a slice moves to `## closed` when its status becomes terminal and never moves back. The [development workflow](../guides/development-workflow.md#pick-up-current-work) owns the status vocabulary and the pick-up procedure, and the [charter](./charter.md) owns the appetite unit.

## in flight

- 002 secure-session-storage — shaped — 3 implementation sessions — after 001
- 003 supervised-child-runtime — shaped — 3 implementation sessions — after 002
- 004 profile-composition — shaped — 4 implementation sessions — after 002 and 003; 003 serializes runtime before composition
- 005 account-authentication — shaped — 4 implementation sessions — after 003 and 004; 004 serializes composition before accounts
- 006 proxy-guide-and-research — shaped — 1 implementation session — after 003, 004, and 005
- 007 comprehensive-doctor — shaped — 1 implementation session — after 004, 005, and 012; 012 gives the report renderer its contract
- 008 cli-artifacts — shaped — 1 implementation session — after 005 and 007
- 009 release-readiness — shaped — 1 implementation session — after 006, 007, and 008

## closed

- 001 native-passthrough-foundation — done — 4 implementation sessions
- 010 native-repository-gates — done — 1 implementation session — replaced four shell check-scripts
- 011 entry-point-and-contract-repair — done — 3 implementation sessions — repaired contracts 001 claimed but did not meet
- 012 human-presentation — done — 2 implementation sessions — decided the colour contract every later renderer applies
