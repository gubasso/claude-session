# ADR-0117: Supply the child's plugins from a read-only seed

## Context and Problem Statement

[ADR-0106](./ADR-0106-supply-child-assets-from-one-tree.md) excluded `plugins/` because nothing needed it. Something does now. The child keeps every plugin fact under the one directory [ADR-0102](./ADR-0102-key-child-state-by-terminal.md) made per session, so each launch rebuilds them from nothing and none loads.

## Considered Options

- Link one shared `plugins/` tree, as the assets are.
- Declare the plugins in the composed settings and let the child install them.
- Prepend the child's plugin flag to the argument vector.
- Point the child at a read-only seed, answering the directory's first-run question at launch.

## Decision Outcome

Chosen option: the read-only seed at `<data>/plugin-seed/`, named to the child by `CLAUDE_CODE_PLUGIN_SEED_DIR`.

Linking was measured and rejected: the child records a marketplace's install location as an absolute path and follows it literally, so a shared tree reports the plugin as uncached. That is ADR-0106's stated reason with the measurement behind it, and its exclusion stands.

Settings alone was measured and rejected: the child reconciles the declared plugins before registering the seed, so a new directory loads none and only a second launch works, which this wrapper never has. The seed is therefore paired with a launch-time copy of the state it holds, in the shape [ADR-0105](./ADR-0105-seed-a-session-at-launch.md) already uses; the wrapper copies two files as bytes and parses neither.

The argument vector was rejected because no obligation requires it: [ADR-0028](./ADR-0028-pass-composed-settings-with-the-native-flag.md)'s prefix answers a need the environment could not, and the environment answers this one.

## Consequences

- Good: one read-only tree reaching every session, with no writer but the user.
- Good: a seeded marketplace is located by probing, so its content needs no path agreement.
- Bad: a third variable, two more carried filenames, and a copy the child diverges from.
- Bad: a plugin installed inside a session is lost with it, which the guide says plainly.
- Bad: the copied state names its cache by absolute path, so a moved seed loads nothing.

## Status

Implemented

Amends [ADR-0106](./ADR-0106-supply-child-assets-from-one-tree.md), whose exclusion is unchanged. Enacted in [the plugin service](../../src/services/plugins.rs) and [the guide](../guides/supplying-child-plugins.md). Shaped by [038](../plan/slices/038-child-plugin-seed/README.md).
