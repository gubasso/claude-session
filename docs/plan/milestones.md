# Milestones

This is the only delivery-status surface. Work proceeds in dependency order.

| id  | slice                                                                                     | status | appetite                  | depends on    | note                                               |
| --- | ----------------------------------------------------------------------------------------- | ------ | ------------------------- | ------------- | -------------------------------------------------- |
| 001 | [native passthrough foundation](./slices/001-native-passthrough-foundation/README.md)     | done   | 4 implementation sessions | none          |                                                    |
| 002 | [secure session storage](./slices/002-secure-session-storage/README.md)                   | shaped | 3 implementation sessions | 001           |                                                    |
| 003 | [supervised child runtime](./slices/003-supervised-child-runtime/README.md)               | shaped | 3 implementation sessions | 002           |                                                    |
| 004 | [profile composition](./slices/004-profile-composition/README.md)                         | shaped | 4 implementation sessions | 002, 003      | Adds 003 to serialize runtime before composition.  |
| 005 | [account authentication](./slices/005-account-authentication/README.md)                   | shaped | 4 implementation sessions | 003, 004      | Adds 004 to serialize composition before accounts. |
| 006 | [proxy guide and research](./slices/006-proxy-guide-and-research/README.md)               | shaped | 1 implementation session  | 003, 004, 005 |                                                    |
| 007 | [comprehensive doctor](./slices/007-comprehensive-doctor/README.md)                       | shaped | 1 implementation session  | 004, 005, 012 | Adds 012 so the report renderer has its contract.  |
| 008 | [CLI artifacts](./slices/008-cli-artifacts/README.md)                                     | shaped | 1 implementation session  | 005, 007      |                                                    |
| 009 | [release readiness](./slices/009-release-readiness/README.md)                             | shaped | 1 implementation session  | 006, 007, 008 |                                                    |
| 010 | [native repository gates](./slices/010-native-repository-gates/README.md)                 | done   | 1 implementation session  | 001           | Replaces four shell check-scripts.                 |
| 011 | [entry point and contract repair](./slices/011-entry-point-and-contract-repair/README.md) | done   | 3 implementation sessions | 001           | Repairs contracts 001 claimed but did not meet.    |
| 012 | [human presentation](./slices/012-human-presentation/README.md)                           | shaped | 2 implementation sessions | 001           | Carries the colour decision through to bytes.      |
