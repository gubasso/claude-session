# ADR-0058: Behave as stock `claude` unless a wrapper feature requires otherwise

## Context and Problem Statement

The passthrough contract is stated for argv ([ADR-0002](./ADR-0002-verbatim-argv-passthrough.md)) and for exit status ([ADR-0005](./ADR-0005-exit-code-taxonomy.md)), but a wrapper is observable in ways neither covers: terminal and signal behaviour, job control, and the files a run leaves on disk. Each of those has been argued from first principles at the point it came up, which invites re-litigation and, worse, lets the wrapper acquire failure modes the wrapped program does not have.

## Considered Options

- A standing default: every user-observable behaviour matches stock `claude` unless a wrapper feature requires the difference.
- Keep deciding transparency case by case, per reference page.
- Full transparency with no exceptions, which means `exec` and no wrapper features at all.

## Decision Outcome

Chosen option: **a standing default** — the user is running `claude`, and any divergence is a cost a named wrapper feature has to justify.

Two exception classes, and only these: the wrapper's own verbs and its short denylist of intercepted flags ([ADR-0003](./ADR-0003-reserve-a-small-wrapper-cli-surface.md), [ADR-0043](./ADR-0043-match-wrapper-flags-by-exact-leading-spelling.md)), and the supervision that spawn-and-wait obliges ([ADR-0004](./ADR-0004-spawn-and-wait-child-supervision.md)).

The corollary is the load-bearing part: **the wrapper introduces no failure mode the child does not have.** State a run leaves behind — a lock, a temp file, a published process id — is repaired by the next run rather than blocking it, because a `claude` killed by `SIGKILL` never poisons the next `claude`.

## Consequences

- Good: a tiebreaker for decisions with no argv in them, so "what does stock `claude` do" replaces a fresh argument each time.
- Good: it decides in advance that wrapper-invented state cannot become a wrapper-invented error.
- Bad: the rule is only as good as the measurements behind it, so a behaviour claim about the child belongs in [research tracking](../reference/research-tracking.yaml) rather than being asserted as permanent.
- Bad: a genuinely useful feature that diverges visibly now needs its own record naming the divergence.

## Status

Accepted
