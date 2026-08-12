# ADR-0057: Build the child environment by prefix scrub and marker

## Context and Problem Statement

The environment is the wrapper's only non-argv channel to the child, and it carried the marker half of the recursion guard ([ADR-0055](./ADR-0055-compare-executable-identity-by-device-and-inode.md)) without ever being decided. It was an unordered table of six operations, two of which compose correctly in only one order, plus a "composed injections" row advertising a surface that existed in neither the flag table nor the configuration schema.

## Considered Options

- Scrub the `CLAUDE_SESSION_` prefix, set the marker, inherit everything else
- Clear the environment and restore an allowlist, as `sudo` does
- The above, plus a configured injection map
- The above, plus an `--env KEY=VAL` flag

## Decision Outcome

Chosen option: prefix scrub plus marker — the child is a normal program, so the wrapper removes its own namespace and adds nothing the child cannot already be given. The ordered algorithm is in [process runtime](../reference/process-runtime.md#child-environment); the scrub is by prefix rather than by a known-key list, so a wrapper input cannot leak to a nested reader.

An allowlist is rejected: it is a privilege boundary, which this is not, and it would strip the ambient authentication [accounts](../reference/accounts.md) promises to leave alone.

Injection is rejected because the proxy seam is inheritance — the child reads its base URL from the environment, and the wrapper already passes it through, so a surface would buy nothing ([ADR-0048](./ADR-0048-build-for-a-present-need.md)). A flag would additionally cost a denylist row, unreachable for the child forever after ([ADR-0002](./ADR-0002-verbatim-argv-passthrough.md)). If revisited: wrapper keys win over injected ones, keys are validated for emptiness, `=`, and NUL — Rust validates neither, splitting the first wrongly and failing the spawn on the second — the code is `Config`, and git's convention of a bare name meaning "unset" is the encoding to copy.

## Consequences

- Good: one removal, three additions, and no mechanism whose failure modes need codes.
- Bad: a per-profile base URL needs a shell wrapper, since configuration cannot set one.

## Status

Implemented

Enacted by [`src/services/child.rs`](../../src/services/child.rs).

Amended by [ADR-0089](./ADR-0089-carry-a-child-owned-fact-only-against-an-obligation.md), naming its obligation, and [ADR-0092](./ADR-0092-namespace-apart-from-the-predecessor.md), moving the prefix.
