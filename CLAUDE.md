# CLAUDE.md

Claude Code specific guardrails for this repository. For broader
cross-agent context (tech stack, build commands, negative scope), see
[AGENTS.md](AGENTS.md).

## About this repo

`claude-session` is a **public** bash CLI that isolates `CLAUDE_CONFIG_DIR`
per terminal. It is a generalized port of a personal wrapper script — all
organization-specific values must be user-configurable, never hard-coded.

## Canonical specs

Design intent lives in [`docs/`](docs/). Before changing behavior,
read the relevant spec file:

- `docs/architecture.md` — module layout, startup flow, session-dir lifecycle.
- `docs/commands.md` — user-facing surface (every subcommand, flag, exit code).
- `docs/config.md` — config file format, env var reference.
- `docs/features.md` — scope of v1 and what's deferred.

## Before committing

Run:

```sh
just check
```

This runs the full lint + test matrix. **Linting always goes through
`pre-commit`** — never invoke `shellcheck`, `shfmt`, or other linters
directly. If `pre-commit run --all-files` is clean, linting is clean.

## Public-repo hygiene (hard rules)

- Never commit organization-specific cloud project IDs (e.g. any
  `vertex-ai-*`, internal GCP/AWS project slugs).
- Never commit OAuth tokens, API keys, or secrets of any kind.
- Never commit paths to personal secret stores (e.g. `gopass show …`).
- Never commit profile names that identify individuals or employers.
  Example names must be generic: `default`, `work`, `experiment`, `vertex`.
- If a command, flag, or env var would need personal data to work,
  that personal data belongs in the user's
  `~/.config/claude-session/config.env` — not in the repo.

Any value that could be sensitive should be introduced as a generic env
var documented in `docs/config.md`, with a placeholder in examples
(e.g. `<your-gcp-project-id>`).
