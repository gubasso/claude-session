# ADR-0055: Compare executable identity by device and inode

## Context and Problem Statement

The recursion guard's self-check compared the canonicalized resolved child path against the canonicalized path of the wrapper's own executable. Canonicalization resolves symbolic links, `.`, and `..`, but a hard link is not a link the filesystem can resolve — two names for one inode canonicalize to two different paths. `ln claude-session claude` is a plausible packaging shape, and it defeats a string comparison.

## Considered Options

- Compare device and inode of both resolved paths
- Compare canonical path strings
- Compare `argv[0]` or the file name, as ccache does
- Count re-entries with an environment counter, as rustup's `RUST_RECURSION_COUNT` does

## Decision Outcome

Chosen option: **device and inode** — it is the filesystem's own answer to "same file", and it catches the hard link that path equality misses.

Self-identity comes from `std::env::current_exe`, which on the supported Linux target reads `/proc/self/exe`. The comparison's two blind spots — a byte-for-byte _copy_ of the wrapper, and a binary replaced on disk mid-run — are exactly what the marker variable covers, which is what makes "neither guard is sufficient alone" precise rather than asserted.

`argv[0]` is rejected because this wrapper may legitimately be installed _as_ `claude`, so the name discriminates nothing. A counter is rejected because rustup tolerates legitimate proxy nesting and this wrapper does not: there is one child, any re-entry is a fault, and a bound would be a knob with no present need ([ADR-0048](./ADR-0048-build-for-a-present-need.md)).

## Consequences

- Good: a hard-linked install is refused rather than forking until something breaks.
- Good: the two guards now have stated, non-overlapping blind spots.
- Bad: `claude-session` invoked from inside a Claude Code session inherits the marker and refuses to run at all. That is the guard working; it is stated in [process runtime](../reference/process-runtime.md#recursion-guard) and has a test, rather than being left to discovery.
- Bad: two `stat` calls on every invocation, including passthrough.

## Status

Accepted
