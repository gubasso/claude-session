# Agent Guidelines — claude-session

This file is the single source of truth for how agents work in this project. `CLAUDE.md` imports it with `@AGENTS.md`, so both Claude Code and the `AGENTS.md`-native tools (Codex, Cursor, and others) read one authored document.

## The Project

`claude-session` is a Rust CLI binary crate that wraps the `claude` command. Three hard contracts bind every change:

- **Never break native `claude` passthrough.** Anything the wrapped `claude` accepts must keep working through the wrapper, with the same semantics. A change that alters passthrough behaviour is a breaking change and needs an ADR.
- **Stay XDG-compliant.** Config, state, data, and cache go to their `XDG_*` locations (with the documented defaults); never write to `$HOME` directly or to hard-coded personal paths.
- **Stay self-contained**, in the sense the next section defines. This is load-bearing here, not boilerplate: the wrapper must not depend on a personal or external tree to build or run.

The engineering specifications these contracts are worked out in live under `docs/`, indexed by [`docs/README.md`](./docs/README.md). They are normative design for code that has not been written yet — read the specification that owns a rule before implementing against it.

[Project governance](./docs/reference/project-governance.md#rule-ownership-and-enforcement) maps each binding rule to its canonical owner, rejecting mechanism, and change protocol.

<!-- self-containment -->

## Self-Containment

Non-negotiable: this project is self-contained. The knowledge it depends on is held in-repo. An external reference is allowed only as a public link or citation for further reading — never as a load-bearing dependency on a resource outside the repository, and in particular never on an external, local, personalized, or mutating repository, path, or tool. If an external document, repo, or personal path is required to understand, build, or operate this project, copy its essential knowledge into the repository (a doc, an ADR, or an inline comment) so the repo stays complete on its own.

## Scope

Non-negotiable: build for a present need — YAGNI. A flag, subcommand, configuration key, or abstraction earns its place by a use this project has today, never by symmetry with a sibling, by completeness of a table, or by a use someone might have later. The cost of a speculative surface is not writing it: every surface is a contract that has to be specified, tested, documented, and kept working across every later passthrough and XDG change, and removing one is a breaking change. Prefer one command that answers the whole question to several that each answer a slice of it. A need that is real but not yet present is recorded as a rejected option in the ADR that considered it, so it is not re-debated, rather than shipped early.

The test for a surface that does exist is what it **discriminates**: a subcommand distinguishes itself from its siblings, a flag distinguishes one invocation's behaviour from another's. So a namespace verb with a single subcommand collapses to the bare verb, and a flag is declared on the invocations that act on it rather than globally ([ADR-0051](./docs/decisions/ADR-0051-let-every-surface-element-discriminate.md)).

## Decisions

Non-negotiable: record every significant, hard-to-reverse decision as an ADR under the project's decisions directory, one decision per file, using the MADR-minimal `template.md`, so the rationale lives with the project. Accepted ADRs are not deleted; a changed decision gets a new superseding ADR. Exact status authority is in [project governance](./docs/reference/project-governance.md#decision-status).

ADRs live in `docs/decisions/`, named `ADR-NNNN-short-title.md` from the `template.md` beside them. Anything that changes the passthrough contract, the XDG layout, the CLI surface, or a dependency that is hard to back out of earns one.

## Planned Work

Implementation work is organized under `.implementation-plans/`, and `.implementation-plans/queue-plans.yaml` is the queue's source of truth — read it before starting work to find what is queued, in progress, or done, and update a plan's status there rather than inferring it from the tree. Individual plans live in `.implementation-plans/plans/`; a plan describes the work, while an ADR records a decision the work rests on.

**A round file is not a specification.** It describes work to do; the durable contracts live under `docs/`. When a round and a specification disagree, the specification wins and the round is corrected — unless the round is actually right, in which case update the specification _and_ the ADR carrying that decision. A genuinely open question becomes a new ADR with status `Proposed` rather than two live claims in one repository.

## Documentation Maintenance

Documentation is organized by **reader need**, not by topic. Four zones, each making one promise: `docs/decisions/` records why a choice was made, `docs/explanation/` builds a mental model, `docs/reference/` gives exact lookup, `docs/guides/` gives ordered tasks. A topic directory goes _inside_ a zone, never as a sibling of the zones. `docs/README.md` is an index and never a fifth zone. A zone is created by its first real document, never by a placeholder. Recorded in [`docs/decisions/ADR-0012-docs-architecture.md`](./docs/decisions/ADR-0012-docs-architecture.md).

Documentation is written **ahead of** the code it specifies, and no zone has a page cap — the specifications drive implementation, so a missing page is work outstanding rather than a decision ([ADR-0036](./docs/decisions/ADR-0036-write-the-specifications-before-the-code.md)).

- **One fact, one home.** Write a durable fact once, in the document that owns it, and link from everywhere else. The test is whether deleting a mention elsewhere would leave the canonical statement intact; if not, the mention is a second source of truth. Restatement is how documentation drifts.
- **Lean prose.** Prefer a list, a table, or a runnable command to a paragraph. State the fact; do not argue for it, weigh it, or restate it in a closing sentence. Keep a sentence of rationale only where its absence invites a wrong change. Depth belongs in the document that owns the topic — link there instead of summarizing, since a summary is a second source of truth that drifts.
- **No stubs.** A file whose whole content is a pointer is deleted, not kept: redirect shims, `superseded — see X` notes, placeholder pages, and empty sections. Fix the inbound links instead. This is the same rule as "a zone is created by its first real document", applied to files.
- **ADRs stay lean.** About 350 words in the template's five sections, one decision per file, one status from `{Proposed, Accepted, Implemented, Superseded, Deprecated, Rejected}`. The budget is a splitting signal, not a tripwire: a record at or under 450 words by `wc -w` is fine as it stands, and one over 450 is trimmed or split in the change that notices it — moving worked detail to the reference page the record links to — never logged as a task ([ADR-0041](./docs/decisions/ADR-0041-budget-adr-length-with-a-margin.md)).
- **ADRs are never deleted.** A decision that stops being true is `Superseded` by a new record with a forward link; one whose context evaporated with no successor is `Deprecated`; one partly changed keeps its status and gains an `Amended by ADR-NNNN` line. A rejected option worth not re-debating stays as a `Rejected` record. [Decision status](./docs/reference/project-governance.md#decision-status) defines which values are current authority.
- **Directory structure is owned by the filesystem, not by prose.** A `README.md` (or `AGENTS.md`) explains a directory's purpose — its domains, concepts, and rules — and never maintains a hand-copied file tree, which drifts the moment a file is added or renamed. When a listing aids discovery, give each entry a purpose, not a bare path the filesystem already shows. An auto-generated table of contents is the exception, since the generator keeps it in sync.
- **Comments are documentation only when load-bearing.** Keep a comment that carries rationale, an invariant, a boundary condition, or an external constraint; delete one that narrates what the code plainly does, or replace it with a better name. Where an ADR governs the code, name it in the comment.
- **This file is authored, not generated.** It carries no frontmatter and no source map by choice: it is written to be read, not synthesized from the documents it points at. The digest convention — frontmatter, a token estimate, a regenerate-on-change rule — belongs to files that summarize a directory, which this one does not. Do not "fix" it toward that shape.
- **Drafts live in the gitignored `.draft/`.** Promotion out of it is a rewrite into the owning document, not a move — a draft is written for its author, a specification for the next reader. Delete the draft once its substance has shipped.
- Perishable facts — anything externally owned, such as the wrapped binary's behaviour — are registered in [`docs/reference/research-tracking.yaml`](./docs/reference/research-tracking.yaml) with a cadence, rather than being asserted as permanent.

## Working Conventions

The exact owner, enforcement, and change protocol for binding project rules is mapped in [project governance](./docs/reference/project-governance.md#rule-ownership-and-enforcement).

- Keep changes scoped and reversible; prefer editing existing files over adding new ones.
- Documentation and rationale live beside the code they describe.
- Run the project's own lint and test tasks before proposing changes. The gates are wired into `.pre-commit-config.yaml` — rustfmt, clippy, nextest, cargo-audit, cargo-deny, taplo, typos, gitleaks, ripsecrets, and cargo-machete — so `pre-commit run --all-files` is the single command that reproduces CI's verdict locally. Fix what the hooks report; do not bypass them.
- Commit messages follow Conventional Commits, linted by `committed` against `committed.toml`: lowercase description, no trailing period, and every line (subject included) at or under 72 characters. Tune `committed.toml` rather than the hook config.
- The pinned toolchain in `rust-toolchain.toml` is authoritative; the `flake.nix` devShell provides the surrounding tools. Do not depend on whatever happens to be installed on the host.
