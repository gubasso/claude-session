# AGENTS.md

Cross-agent context for AI coding agents (Codex CLI, Claude Code, Cursor,
Gemini CLI, and any tool that reads `AGENTS.md`).

## What this project is

`claude-session` is a bash CLI that wraps the `claude` binary to give each
terminal its own isolated `CLAUDE_CONFIG_DIR`, while transparently sharing
portable config (auth, settings, skills, agents, rules, commands, hooks)
back to `~/.claude/`. It prevents cross-terminal state bleed in Claude Code.

It is a **public repository**. No organization identifiers, personal project
IDs, OAuth tokens, secret-store paths, or profile names tied to an employer
or individual may be committed. When in doubt, generalize into a config
variable.

## Tech stack

- **Language**: bash 4.4+ (`set -euo pipefail`, `shopt -s inherit_errexit`).
- **Build / task runner**: [`just`](https://just.systems/) (see `justfile`).
- **Tests**: [`bats-core`](https://bats-core.readthedocs.io/) with
  `bats-support` / `bats-assert` / `bats-file` as git submodules under
  `test/test_helper/`.
- **Linting**: [`shellcheck`](https://www.shellcheck.net/) and
  [`shfmt`](https://github.com/mvdan/sh), orchestrated through
  [`pre-commit`](https://pre-commit.com/). **Never invoke linters directly;
  always run them through pre-commit.**
- **Man page**: [`scdoc`](https://git.sr.ht/~sircmpwn/scdoc) source in
  `man/*.scd`, compiled to `.1` by `just man`.
- **Runtime dependencies**: `jq` (settings overlay merge), `flock`
  (exit-time sync under lock).

## Build, test, lint

Before any PR:

```sh
just check        # runs lint + test (unit + integration)
```

Subsets:

```sh
just test-unit         # fast bats unit tests
just test-integration  # bats integration tests (touch filesystem)
just lint              # pre-commit run --all-files
just man               # build man page
```

## Where things live

- Source: `bin/claude-session` (entry shim) + `lib/` (loader, core,
  helpers, `commands/`, `functions/`).
- Specs (source of truth for design intent): `docs/`.
- Start with `docs/architecture.md` for the module layout and startup
  flow, then `docs/commands.md` for the user-facing surface.
- Full per-subcommand reference: `docs/commands.md`.
- Config file format and env vars: `docs/config.md`.
- Contributor / dev workflow: `docs/development.md`.

## Conventions

- **Commit style**: [Conventional Commits](https://www.conventionalcommits.org/).
- **Namespace**: public functions are `cs::cmd::<n>`, `cs::fn::<n>`,
  `cs::helpers::<n>`. Private helpers (same-file only) are `__<n>`.
- **One public function per file** under `lib/commands/` and
  `lib/functions/`; filename mirrors the function name.
- **Every `.sh` under `lib/`** starts with `# shellcheck shell=bash`
  followed by `: 'desc: <one-line description>'` (harvested by help/man
  generators — see `docs/development.md`).
- **stdout vs stderr**: data → stdout, progress/warnings/errors → stderr.
  `printf` over `echo`. No ANSI escape codes unless `[[ -t 1 ]]`.
- **Error shape**: three-part (`What went wrong:` / `How to fix:` /
  `Next:`) on stderr; non-zero exit code. Template in
  `docs/architecture.md`.
- **Exit codes**: documented in `docs/commands.md`; agents may rely on
  them as a stable contract.

## Negative scope

- Do **not** commit configuration containing organization identifiers
  (cloud project IDs, internal hostnames, secret-store paths), OAuth
  tokens, or profile names that identify individuals / employers.
- Do **not** add an MCP shim, a SKILL.md, or a cross-agent skill package
  in this repo — those are planned as separate downstream repos.
- Do **not** add `--json` output, `schema <type>`, or verify/validate
  subcommands in v1. They are deferred to v1.1+ (see
  `docs/features.md` → "Deferred").

## References

The design draws on two internal references:

- Bash CLI project layout, strict mode, XDG install, testing — see
  `docs/architecture.md` and `docs/development.md`.
- Agent-facing CLI surface (`--help`, `usage`, `doctor`, error shape,
  exit codes, config precedence) — see `docs/commands.md` §"Per-subcommand
  reference".
