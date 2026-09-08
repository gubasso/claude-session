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

## Delegation

The child owns its own schema, its own validation, and its own error messages. Carry one of its names, schemas, or behaviours only when a wrapper obligation cannot be met without it: launching it, avoiding a collision with a spelling it already claims, or keeping the wrapper from over-claiming its own effect. Register every carried identifier in [child facts](./docs/reference/child-facts.yaml) against that obligation; a carry meeting none of the three is removed rather than recorded, and a contested one becomes an open question ([ADR-0089](./docs/decisions/ADR-0089-carry-a-child-owned-fact-only-against-an-obligation.md)). This binds shaping as much as implementation: a slice that can only be delivered by importing a child-owned fact is not a slice to shape.

## Decisions

Record each significant, hard-to-reverse choice in one filled file under `docs/decisions/`, using [the template](./docs/decisions/template.md). To change a binding rule, follow its current owner first; add or supersede an ADR only when the significance test is met, update the owner and rejecting mechanism together, and label review-only enforcement honestly.

## Current Work

[Milestones](./docs/plan/milestones.md) is the only delivery-status surface, and it carries live work under `## in flight` and terminal work under `## closed`. Open the `active` slice, or promote the first `shaped` line under `## in flight`, which execution order makes the next slice whose named predecessors are closed. Then load only the sources its `Governed by` section names. The slice entry is the execution contract; `tasks.md`, when present, only records cross-session checklist progress.

## Documentation Maintenance

- Use five reader-need zones: `decisions/` records why, `explanation/` builds current mental models, `reference/` provides exact lookup, `guides/` provides ordered tasks, and `plan/` carries perishable delivery state.
- Apply ARID: one durable fact, one owner, and links from every other mention. Let the filesystem own directory structure; indexes explain purpose rather than copying a tree.
- Create a subsystem explanation page only when implemented or already-substantive design needs a living model. Never create placeholders.
- Keep each filled ADR at or below 350 whole-file words, in the template's five sections, with a stable id and one status from `Ideation`, `Proposed`, `Accepted`, `Implemented`, `Deprecated`, `Superseded`, or `Rejected`.
- Only `Accepted` and `Implemented` ADRs are current authority. Other statuses may explain history but cannot govern a slice.
- Never delete an ADR from `Proposed` onward. Supersede a changed decision, deprecate an evaporated context, retain a rejected option worth remembering, and add `Amended by ADR-NNNN` for a partial change.
- Give each slice a fixed appetite and non-negotiable core. Record a dated revision only when work has begun and `Goal`, `Core`, `Appetite`, or `Acceptance` changes; cutting the end of `In scope` is not a revision.
- Every slice has one directory entered through `README.md`, titled `# <id> — <Title>`, with `Goal`, `Appetite`, `Core`, `In scope`, `Out of scope`, `Governed by`, `Acceptance`, `Rabbit holes`, `Done when`, and `Revisions`, in that order.
- Gate every fixed shape with one `MD043` array under `.markdownlint/`, applied by its own `md-*` entry in `.pre-commit-config.yaml` — the slice `README.md`, `milestones.md`, and the decision records. Amend the array to add a section; never copy one into a document, and never name `MD043` in `.markdownlint-cli2.jsonc`, which is merged over the shape and would switch it off silently.
- Keep one milestone line per slice, in the fixed grammar `<id> <slug> — <status> — <appetite>[ — <note>]`. Order `## in flight` by execution, so the top line is the next slice to pick up, and `## closed` by id, because a ledger is read by lookup. A line with a predecessor opens its note with `after <id>[ and <id>]`; every id named there is closed or listed above it, which is what makes the execution order checkable rather than asserted. Move the line to `## closed` in the same change that ends the work, and never move it back.
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
- Satisfy [presentation](./docs/reference/presentation.md) before writing a human-facing byte. Plain text carries the whole meaning, the colour decision is resolved once rather than per renderer, and only the surfaces that page names are coloured. It binds every renderer the project writes, so a new verb reads it instead of inventing an appearance, and a disagreement changes the renderer.
- Add dependencies through the documented procedure and the pinned devShell; do not depend on host tooling.
- Run `just hooks` before proposing changes. It executes both configured hook stages; `pre-commit run --all-files` alone is not the verdict. The exact gate is in [testing and quality](./docs/reference/testing-and-quality.md#the-gate).
- Commit messages use Conventional Commits, a lowercase description, no trailing period, and at most 72 characters on every line.

<!-- BEGIN release-kit -->

## Releases

- This repository runs the release-kit convention. `rk method invariants` states what must stay true.
- An agent here guides and never drives. It reads this convention and tells the operator which step comes next. It takes no git or forge action unless the operator's request named that action. The bounded actions include the following. Create, switch, or delete a branch. Mint a branch at the forge from an issue. Create or remove a worktree. Commit, push, or tag. Open, update, or merge a pull request. A request to change code authorizes the file changes alone.
- Work reaches the trunk only through a squash-merged pull request from a short-lived branch. The branch name is `<type>/<slug>`, whose type matches the squash title's type, or the forge-minted `<issue-id>-<slug>`. Nothing is committed on `master`.
- A request that names an issue starts from the forge's own branch: `rk issue start <issue>` mints it at the forge, seats it, and links it to the issue. Never invent a name for work an issue already names.
- This project works in worktrees: every code-changing branch lives in its linked worktree (`rk worktree add <branch>` creates or adopts it beside the checkout), the main checkout commits nothing, and `rk worktree prune` retires a merged worktree. One branch, one writer.
- The request's title becomes the trunk's commit message, so it MUST be a scoped Conventional Commit. The body carries the context and lands with it. The body names no internal planning artifact and carries no agent attribution. The landed rk-message hook, the forge's body check, and the observed body source hold that rule.
- Every commit follows the same scoped convention. The landed commit-msg hook requires a scope on every one, and the title check holds it to lowercase letters, digits, and `_ . / -`.
- The scope names the area you changed, and reads as `area/subarea` where that is clearer. Prefer a scope this repository already uses, which `git log --format=%s | sed -n 's/^[a-z]*(\([^)]*\)).*/\1/p' | sort -u` lists. Coin a new scope only where no existing one names the area.
- Never author a tag, and never hand-edit a generated artifact workflow.
- Run `rk status` before changing anything under `.github/workflows/` or `.gitlab-ci.yml`, or any file `.release-kit/manifest.json` names.
- The full method is `rk method --list`. The recovery paths are `rk method recovery`.

<!-- END release-kit -->
