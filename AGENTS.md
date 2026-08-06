# Agent Guidelines — claude-session

This is the authored source of truth for agents. `CLAUDE.md` imports it with `@AGENTS.md`.

## The Project

`claude-session` is a Rust CLI wrapper around `claude`. Three hard contracts bind every change:

- Never break native passthrough. Anything the child accepts keeps the same argv, streams, signals, and status through the wrapper.
- Stay XDG-compliant. Resolve config, state, data, and cache through their XDG owners; never write directly under `$HOME`.
- Stay self-contained. Building, operating, and understanding the project must not require a personal, external, or mutating local tree.

The normative engineering documents are indexed by [the documentation index](./docs/README.md). Read the owner of a rule before changing it.

## Self-Containment

Repository knowledge is load-bearing. External material may be a public citation for further reading, never a required local path, repository, or tool. Copy essential knowledge into its in-repository owner before depending on it.

## Scope

Build for a present need. A flag, verb, configuration key, document, or abstraction must discriminate from its siblings through a current use. Prefer one surface that answers the need to several speculative surfaces; record a rejected option instead of shipping it early ([ADR-0051](./docs/decisions/ADR-0051-let-every-surface-element-discriminate.md)).

## Decisions

Record each significant, hard-to-reverse choice in one filled file under `docs/decisions/`, using [the template](./docs/decisions/template.md). To change a binding rule, follow its current owner first; add or supersede an ADR only when the significance test is met, update the owner and rejecting mechanism together, and label review-only enforcement honestly.

## Current Work

[Milestones](./docs/plan/milestones.md) is the only delivery-status surface, and it carries live work under `## in flight` and terminal work under `## closed`. Open the `active` slice, or promote the first `shaped` slice whose named predecessors are closed, then load only the sources its `Governed by` section names. The slice entry is the execution contract; `tasks.md`, when present, only records cross-session checklist progress.

## Documentation Maintenance

- Use five reader-need zones: `decisions/` records why, `explanation/` builds current mental models, `reference/` provides exact lookup, `guides/` provides ordered tasks, and `plan/` carries perishable delivery state.
- Apply ARID: one durable fact, one owner, and links from every other mention. Let the filesystem own directory structure; indexes explain purpose rather than copying a tree.
- Create a subsystem explanation page only when implemented or already-substantive design needs a living model. Never create placeholders.
- Keep each filled ADR at or below 350 whole-file words, in the template's five sections, with a stable id and one status from `Ideation`, `Proposed`, `Accepted`, `Implemented`, `Deprecated`, `Superseded`, or `Rejected`.
- Only `Accepted` and `Implemented` ADRs are current authority. Other statuses may explain history but cannot govern a slice.
- Never delete an ADR from `Proposed` onward. Supersede a changed decision, deprecate an evaporated context, retain a rejected option worth remembering, and add `Amended by ADR-NNNN` for a partial change.
- Give each slice a fixed appetite and non-negotiable core. Record a dated revision only when work has begun and `Goal`, `Core`, `Appetite`, or `Acceptance` changes; cutting the end of `In scope` is not a revision.
- Every slice has one directory entered through `README.md`, titled `# <id> — <Title>`, with `Goal`, `Appetite`, `Core`, `In scope`, `Out of scope`, `Governed by`, `Acceptance`, `Rabbit holes`, `Done when`, and `Revisions`, in that order.
- Pin every fixed-shape document with its own `MD043` heading array in a `markdownlint-configure-file` comment under the H1 — every slice `README.md` and `milestones.md`. `MD043` is off repository-wide because its array is per document; amend an array to add a section, never delete the pin.
- Keep one milestone line per slice, in the fixed grammar `<id> <slug> — <status> — <appetite>[ — <note>]`, ordered by id inside its section. Move the line to `## closed` in the same change that ends the work, and never move it back.
- Until the current slice is implemented, add no specification page and open no ADR outside it. Route new blockers to [open questions](./docs/plan/open-questions.md).
- Implemented slice acceptance may append `-> <nextest-test-id>` only when `scripts/check-acceptance-tests` resolves the exact ID.
- A `tasks.md` contains only `# Tasks` and flat checkboxes; delete it when the slice becomes `done`. `design.md` is forbidden. `requirements.md` needs the explicit gate marker and replaces, rather than duplicates, acceptance.
- Track external-system bugs under `docs/reference/known-issues/` only when the first real case exists; expand while active and collapse after resolution.
- Track perishable facts in [research tracking](./docs/reference/research-tracking.yaml) with exactly `path`, `last_checked`, `cadence`, `why`, `revalidate`, and `dependents`.
- Use no bold or italics. Put identifiers in inline code, use language-labelled fenced blocks, and keep comments only for rationale, invariants, boundary conditions, or owning ADR links.
- Keep drafts in ignored `.draft/`. Promotion is a rewrite into the owner, followed by deletion of the draft.

## Working Conventions

- Keep changes scoped and reversible; documentation and rationale live beside what they describe.
- Use post-2018 Rust module form, `pub(crate)` by default, typed layer errors, one immutable `AppContext`, and OS strings at process boundaries. Exact rules live in [coding conventions](./docs/reference/coding-conventions.md).
- Add dependencies through the documented procedure and the pinned devShell; do not depend on host tooling.
- Run `just hooks` before proposing changes. It executes both configured hook stages; `pre-commit run --all-files` alone is not the verdict. The exact gate is in [testing and quality](./docs/reference/testing-and-quality.md#the-gate).
- Commit messages use Conventional Commits, a lowercase description, no trailing period, and at most 72 characters on every line.
