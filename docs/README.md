# Documentation

`claude-session` ships its native-passthrough foundation. These documents distinguish that implemented foundation from later specified slices.

| Zone                          | Purpose                                                                   |
| ----------------------------- | ------------------------------------------------------------------------- |
| [Decisions](./decisions/)     | Frozen choices, alternatives, consequences, and lifecycle state.          |
| [Explanation](./explanation/) | Current mental models for architecture, wrapping, isolation, and testing. |
| [Reference](./reference/)     | Exact contracts, tables, schemas, and tracked external facts.             |
| [Guides](./guides/)           | Ordered setup, development, and release procedures.                       |
| [Plan](./plan/)               | Charter, milestone status, bounded slices, and blocking questions.        |

## Start here

- For the product shape, read [architecture](./explanation/architecture.md) and the [wrapper model](./explanation/wrapper-model.md).
- For exact behavior, begin with the [CLI surface](./reference/cli-surface.md), [process runtime](./reference/process-runtime.md), [XDG storage](./reference/xdg-storage.md), and [exit codes](./reference/exit-codes.md).
- For anything a user reads, follow [logging and output](./reference/logging-and-output.md) for the destination and [presentation](./reference/presentation.md) for the appearance.
- For current work, open [milestones](./plan/milestones.md) and follow the live work under `## in flight`.
- For the one-time machine setup that puts your own skills and agents in every session, use [supplying child assets](./guides/supplying-child-assets.md).
- For contribution procedure, use the [development workflow](./guides/development-workflow.md) and [testing gate](./reference/testing-and-quality.md#the-gate).
- For release operations, use the [release guide](./guides/releasing.md) and [release contract](./reference/release-workflow.md).
- For externally owned facts, consult [research tracking](./reference/research-tracking.yaml) before relying on a measured child behavior.
- For a workaround that looks unexplained, read [known issues](./reference/known-issues/), which records the upstream defect behind it.
