# ADR-0078: Adopt the seven-state decision lifecycle

## Context and Problem Statement

The decision log used six recorded states but had no low-commitment state for a candidate decision whose options were already understood. Status syntax and authority rules also lived in a duplicate governance page.

## Considered Options

- Keep the six existing values.
- Add `Ideation` to the closed lifecycle.
- Track all hesitation only as open questions.

## Decision Outcome

Adopt `Ideation`, `Proposed`, `Accepted`, `Implemented`, `Deprecated`, `Superseded`, and `Rejected`. Never-delete begins at `Proposed`; an `Ideation` record may still be discarded before proposal.

`Ideation` captures a candidate decision whose options are already nameable and which will become an ADR. An open question instead blocks named work and may exit as an ADR, a slice revision, or a measurement.

Only `Accepted` and `Implemented` records are current authority. Other states may explain history but cannot supply a current rule. The first nonblank line below `## Status` is the status value alone; relationship or evidence prose follows in a separate paragraph.

## Consequences

- Hesitation with known options can mature without pretending approval.
- The question register remains the home for blockers whose outcome type is not known.
- Status authority and parsing syntax now live with the lifecycle decision and template.
- The larger vocabulary requires a deterministic checker.

## Status

Accepted
