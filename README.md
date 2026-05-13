# claude-session

> Per-terminal session isolation for Claude Code, as a proper agent-friendly bash CLI.

`claude-session` is a thin wrapper around the `claude` binary that gives each
terminal its own `CLAUDE_CONFIG_DIR`, with shared portable config (auth,
settings, skills) transparently linked back to `~/.claude/`. Session-specific
state (history, projects, todos, plans) stays isolated per terminal.

## What it does

- **Per-terminal isolation**: each `tty` (e.g. `pts/0`, `pts/1`) gets its own
  session directory under `$XDG_RUNTIME_DIR/claude-session/sessions/`
  (or the XDG-spec `${XDG_STATE_HOME:-$HOME/.local/state}/claude-session/sessions/`
  fallback when no runtime dir is available); no cross-bleed
  of prompts, todos, or project metadata between terminals.
- **Layered profile composition**: declare profiles with
  `profiles/<name>.yaml`, compose ordered `settings/<layer>.json`
  files at startup via `jq`, and export merged profile env from the
  composed `env` block.
- **Configurable OAuth and post-exit hooks**: run any command to supply an
  OAuth token at startup (gopass, pass, Bitwarden, 1Password, age — see
  [docs/auth.md](docs/auth.md)), run any command after `claude` exits (cost
  sync, analytics, cleanup). Both optional; nothing personal ships in the repo.
- **XDG-aware install**: user install under `~/.local/`, system install under
  `$PREFIX/`, all paths follow the XDG Base Directory spec.
- **Scriptable / agent-friendly**: per-subcommand `--help`, three-part error
  shape, `doctor` subcommand for structured health checks, `usage` for full
  command-tree introspection, stable exit codes, no ANSI on non-tty output.

## Why it exists

When several terminals attach to the same machine (or devcontainer) and all
run Claude Code, they share `~/.claude/` — and with it, in-flight prompts,
session history, todo lists, and project metadata. State from one terminal
leaks into another, prompts reappear out of context, and todos become chaotic.

`claude-session` fixes this by giving each terminal a dedicated
`CLAUDE_CONFIG_DIR`, with symlinks back to the portable bits (auth tokens,
global settings, skills, agents, rules, commands, hooks) and atomic copy-sync
for files Claude Code rewrites (like `.credentials.json`). The deeper design
lives in [docs/architecture.md](docs/architecture.md).

## Quick start

Install (user scope):

```sh
git clone <this-repo> claude-session
cd claude-session
just install
```

Create a minimal config:

```sh
mkdir -p ~/.config/claude-session
cat > ~/.config/claude-session/config.env <<'EOF'
CLAUDE_SESSION_REAL_CLAUDE=/path/to/claude
EOF
```

Wrap your shell's `claude` invocation:

```sh
# ~/.bashrc or equivalent
alias claude='claude-session run'
```

Verify:

```sh
claude-session doctor
```

Now each new terminal gets its own session directory the first time you run
`claude`.

## Command tree

```
claude-session
├── run [--profile <n>] [-- <claude_args>...]
├── doctor [--verbose]
├── usage
├── config   (show | path | edit)
├── profile  (list | show <name>)
└── session  (list | clean [--older-than <dur>] [--dry-run] [--yes])
```

Full reference: [docs/commands.md](docs/commands.md).

## Configuration

Config precedence is **CLI flags > environment variables > config file**.
The main config file is a global dotenv-style
`~/.config/claude-session/config.env`; profile manifests live under
`~/.config/claude-session/profiles/<name>.yaml` and compose
`~/.config/claude-session/settings/<layer>.json` files. Full reference
and env-var table: [docs/config.md](docs/config.md).

## Paths

| Path | Purpose | Scope |
|---|---|---|
| `${XDG_CONFIG_HOME:-~/.config}/claude-session/config.env` | wrapper global config (`KEY=VALUE`) | user, hand-edited |
| `${XDG_CONFIG_HOME:-~/.config}/claude-session/profiles/<name>.yaml` | profile manifest (ordered `settings-layers`) | user, hand-edited |
| `${XDG_CONFIG_HOME:-~/.config}/claude-session/settings/<layer>.json` | settings layer JSON (composeable) | user, hand-edited |
| `${XDG_CACHE_HOME:-~/.cache}/claude-session/settings.json` | auto-generated persistent runtime layer (lowest-precedence input to compose, written back on exit) | generated, do not hand-edit |
| `$XDG_RUNTIME_DIR/claude-session/` (preferred) | per-machine session root | shared, volatile on logout |
| `${XDG_STATE_HOME:-~/.local/state}/claude-session/` (fallback) | per-machine session root when XDG_RUNTIME_DIR unset | shared, persistent |
| `<root>/sessions/<terminal-id>/` | per-pts session dir (`CLAUDE_CONFIG_DIR`) | per-pts, mode 700 |
| `<session>/settings.json` | composed Claude settings (recomposed each `run`) | per-pts, generated |
| `<session>/.claude-session-compose.json` | sidecar: manifest, layer paths, merged `env` | per-pts, generated |
| `<session>/.claude-session-settings-cache` | sha256 cache key for composer short-circuit | per-pts, generated |
| `<session>/session-meta.json` | schema-v1 metadata (profile, terminal_id, started_at, cwd, session_root, source) | per-pts, generated |
| `<session>/.credentials.json` | OAuth credentials (sync'd in/out under `flock`) | sync |
| `<session>/.claude.json` | symlink → `$HOME/.claude.json` (trust / onboarding / projects map) | home-link |
| `<session>/mcp-needs-auth-cache.json` | MCP auth cache (sync'd) | sync |
| `<session>/{settings.local.json,keybindings.json,CLAUDE.md}` | symlinks → `${CLAUDE_SESSION_SHARED_DIR:-~/.claude}/...` | link |
| `<session>/{skills,agents,rules,commands,hooks,plugins}/` | symlinks → `${CLAUDE_SESSION_SHARED_DIR:-~/.claude}/<dir>` | dir-link |
| `<session>/{backups,cache,sessions,paste-cache,projects,shell-snapshots,session-env,file-history,telemetry,history.jsonl,.last-cleanup}` | Claude Code's own per-CONFIG_DIR state | per-pts, written by Claude |
| `${CLAUDE_SESSION_SHARED_DIR:-~/.claude}/` | upstream Claude Code config; sync'd files live here | shared, persistent |
| `${CLAUDE_SESSION_SHARED_DIR:-~/.claude}/.claude-session.lock` | flock file serializing `sync_files out` across pts | shared, persistent (zero-byte) |

- User edits the versioned inputs under `~/.config/claude-session/`.
- The auto-generated `~/.cache/claude-session/settings.json` is the persistent runtime layer: `/effort` and other in-session mutations land in `$session_dir/settings.json`, then get copied back here on exit.
- On each `run`, the composer merges `cache_settings * base.json * <profile layers...>` and writes to `$session_dir/settings.json`. Versioned layers re-override on every startup, so `effortLevel: "high"` in `base.json` is always the boot-time value, while `/effort medium` in-session still works because Claude Code mutates `$session_dir/settings.json` directly.
- `~/.cache/claude-session/` is scaffolded automatically: `install.sh` (`just install`) pre-creates it on the host so devcontainer bind-mounts succeed on first start, and `uninstall.sh` (`just uninstall`) wipes it. The wrapper also `mkdir -p`s it on every exit before writing the cache, so it self-heals if removed mid-life.

## Requirements

- bash 4.4+
- `jq` (for settings composition)
- `yq` (mikefarah v4+, for manifest parsing)
- `just` (optional; `install.sh` works standalone if you'd rather skip it)

## Documentation

- [docs/architecture.md](docs/architecture.md) — module layout, startup flow, session-dir lifecycle.
- [docs/features.md](docs/features.md) — feature inventory (ported / generalized / new).
- [docs/config.md](docs/config.md) — config file format, env vars, profile system.
- [docs/commands.md](docs/commands.md) — every subcommand, flag, exit code, and example.
- [docs/install.md](docs/install.md) — install matrix, `just` recipes, uninstall.
- [docs/development.md](docs/development.md) — dev setup, tests, linting, CI.
- [AGENTS.md](AGENTS.md) — cross-agent (Codex / Claude Code / Cursor / Gemini) context.
- [CLAUDE.md](CLAUDE.md) — Claude Code guardrails for this repo.

## Contributing

See [docs/development.md](docs/development.md) for environment setup,
strict-mode policy, test layout, and the pre-commit linting workflow.
Run `just check` before opening a pull request.

## License

MIT. See `LICENSE`.
