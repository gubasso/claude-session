# ADR-0041: Budget ADR length with a margin

## Context and Problem Statement

[ADR-0012](./ADR-0012-docs-architecture.md) set an exact 350-word cap on a record. Records landing a few dozen words over it produced standing review items rather than better decisions, and no two counters agreed on the number, since headings, link markup, and the status line are countable or not by taste.

## Considered Options

- An exact cap at 350 words, reviewed by hand whenever a record exceeds it.
- A budget of about 350 words with a margin and a single mechanical counter.
- No length limit, with splitting left to the author's judgement.

## Decision Outcome

Chosen option: a budget with a margin. A record targets about 350 words and is measured by `wc -w` over the whole file, markup included, so the number is reproducible without a convention to remember.

At or under 450 words the record is in budget: nothing to do, nothing to flag. Over 450 it is trimmed or split in the change that notices it — moving worked detail to the reference page that owns it — and never recorded as a task for someone else. The cap keeps its original job as a splitting signal; only the trigger point and the counter change.

## Consequences

- Good: a record is either fine or fixed on sight, so length stops generating backlog.
- Good: `wc -w` gives one answer, so "how long is this?" is not a judgement call.
- Good: 350 stays the target, so records do not drift toward 450 as the new normal.
- Bad: the margin is arbitrary, and a record between 350 and 450 words is longer than the ideal with no mechanism pushing it back down.
- Bad: counting markup inflates a link-dense record against a text-only one.

## Status

Superseded

Superseded by [ADR-0076](./ADR-0076-cap-filled-adrs-at-350-words.md).
