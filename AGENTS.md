# Agent Guidelines — claude-session

This file is the single source of truth for how agents work in this project. `CLAUDE.md` imports it with `@AGENTS.md`, so both Claude Code and the `AGENTS.md`-native tools (Codex, Cursor, and others) read one authored document.

## The Project

`claude-session` is a Rust CLI binary crate that wraps the `claude` command. Three hard contracts bind every change:

- **Never break native `claude` passthrough.** Anything the wrapped `claude` accepts must keep working through the wrapper, with the same semantics. A change that alters passthrough behaviour is a breaking change and needs an ADR.
- **Stay XDG-compliant.** Config, state, data, and cache go to their `XDG_*` locations (with the documented defaults); never write to `$HOME` directly or to hard-coded personal paths.
- **Stay self-contained**, in the sense the next section defines. This is load-bearing here, not boilerplate: the wrapper must not depend on a personal or external tree to build or run.

<!-- self-containment -->

## Self-Containment

Non-negotiable: this project is self-contained. The knowledge it depends on is held in-repo. An external reference is allowed only as a public link or citation for further reading — never as a load-bearing dependency on a resource outside the repository, and in particular never on an external, local, personalized, or mutating repository, path, or tool. If an external document, repo, or personal path is required to understand, build, or operate this project, copy its essential knowledge into the repository (a doc, an ADR, or an inline comment) so the repo stays complete on its own.

## Decisions

Non-negotiable: record every significant, hard-to-reverse decision as an ADR under the project's decisions directory, one decision per file, using the MADR-minimal `template.md`, so the rationale lives with the project. Accepted ADRs are not deleted; a changed decision gets a new superseding ADR.

ADRs live in `docs/decisions/`, numbered `NNNN-short-title.md` from the `template.md` beside them. Anything that changes the passthrough contract, the XDG layout, the CLI surface, or a dependency that is hard to back out of earns one.

## Planned Work

Implementation work is organized under `.implementation-plans/`, and `.implementation-plans/queue-plans.yaml` is the queue's source of truth — read it before starting work to find what is queued, in progress, or done, and update a plan's status there rather than inferring it from the tree. Individual plans live in `.implementation-plans/plans/`; a plan describes the work, while an ADR records a decision the work rests on.

## Working Conventions

- Keep changes scoped and reversible; prefer editing existing files over adding new ones.
- Documentation and rationale live beside the code they describe.
- Directory structure is owned by the filesystem, not by prose. A `README.md` (or `AGENTS.md`) explains a directory's purpose — its domains, concepts, and rules — and never maintains a hand-copied file tree, which drifts the moment a file is added or renamed. When a listing aids discovery, give each entry a purpose, not a bare path the filesystem already shows. An auto-generated table of contents is the exception, since the generator keeps it in sync.
- Run the project's own lint and test tasks before proposing changes. The gates are wired into `.pre-commit-config.yaml` — rustfmt, clippy, nextest, cargo-audit, cargo-deny, taplo, typos, gitleaks, ripsecrets, and cargo-machete — so `pre-commit run --all-files` is the single command that reproduces CI's verdict locally. Fix what the hooks report; do not bypass them.
- Commit messages follow Conventional Commits, linted by `committed` against `committed.toml`: lowercase description, no trailing period, and every line (subject included) at or under 72 characters. Tune `committed.toml` rather than the hook config.
- The pinned toolchain in `rust-toolchain.toml` is authoritative; the `flake.nix` devShell provides the surrounding tools. Do not depend on whatever happens to be installed on the host.
