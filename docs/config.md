# Configuration

`claude-session` draws its runtime settings from three sources. Every
setting has exactly one way to set it at each layer — there are no
duplicate concepts across CLI flags, env vars, and the config file.

## Precedence

Highest priority first:

1. **CLI flags** (e.g. `--profile vertex`, `--config /path/to/config.env`).
2. **Environment variables** (`CLAUDE_SESSION_*` — full list below).
3. **Config file**: `$XDG_CONFIG_HOME/claude-session/config.env`
   (dotenv-style; overridable via `CLAUDE_SESSION_CONFIG_DIR` or the
   `--config <path>` flag).

This matches the convention documented in the internal agent-CLI design
reference: flags > env > file.

## File locations

XDG-aware. The user install targets `$XDG_CONFIG_HOME` (defaulting to
`$HOME/.config`); a system install uses `/etc/claude-session/`.

| Path                                                                          | Purpose                                                                                                        |
|-------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------------------|
| `$XDG_CONFIG_HOME/claude-session/config.env`                                  | Main config. Dotenv: `KEY=VALUE` per line, `#` comments. Sourced at startup before profile resolution.         |
| `$XDG_CONFIG_HOME/claude-session/profiles/<name>.env`                         | Per-profile overrides. Dotenv. Sourced **after** the main config, only if the named profile is active.         |
| `$XDG_CONFIG_HOME/claude-session/profiles/<name>.settings.json`               | Optional JSON overlay for Claude's `settings.json`. Merged atop `$SHARED/settings.base.json` by `jq`.          |
| `$CLAUDE_SESSION_SHARED_DIR/settings.base.json`                               | The Claude-side base settings the overlay extends. Lives in the user's shared Claude config dir (default `~/.claude`). Not created by `claude-session`. |
| `$CLAUDE_SESSION_SHARED_DIR/settings.<profile>.json`                          | Alternative overlay location (legacy-compatible with the source script). Either location works; both are checked. |

The whole config tree can be relocated by setting
`CLAUDE_SESSION_CONFIG_DIR`. This is useful in sandboxed/devcontainer
setups.

## Config-file format

Dotenv: `KEY=VALUE` per line, `#` introduces a line comment, blank lines
are allowed. The file is sourced with `set -a; . "$file"; set +a` so
every assignment becomes an environment variable, no `export` prefix
required.

**Do not** put shell logic in the config file (pipes, command
substitutions, conditionals). It is meant as data. `claude-session`
will load it under `set -a` only; complex initialization belongs in
your shell rc.

Minimal `config.env`:

```
CLAUDE_SESSION_PROFILE=default
```

A more realistic `config.env`:

```
CLAUDE_SESSION_PROFILE=work
CLAUDE_SESSION_OAUTH_CMD=pass show <your-secret-path>
CLAUDE_SESSION_POST_EXIT_CMD=my-cost-sync --session-dir "$CLAUDE_SESSION_DIR"
```

A profile file (`profiles/vertex.env`), with placeholders for values
that must come from your own environment:

```
# Select Vertex AI backend for this profile.
CLAUDE_CODE_USE_VERTEX=1
CLOUD_ML_REGION=us-central1
ANTHROPIC_VERTEX_PROJECT_ID=<your-gcp-project-id>
```

## Environment variable reference

Every variable read by the CLI. Values marked `(unset)` disable the
associated feature entirely.

| Variable                          | Default                                                       | Meaning                                                                                                     |
|-----------------------------------|---------------------------------------------------------------|-------------------------------------------------------------------------------------------------------------|
| `CLAUDE_SESSION_CONFIG_DIR`       | `$XDG_CONFIG_HOME/claude-session`                             | Where config files live. Overriding this moves `config.env` and `profiles/` as a group.                     |
| `CLAUDE_SESSION_SHARED_DIR`       | `$HOME/.claude`                                               | The upstream Claude config dir being isolated. Files in here are what `settings` / `link` / `dir-link` lists point at. |
| `CLAUDE_SESSION_PROFILE`          | `default`                                                     | Active profile name. Selects which `profiles/<name>.env` (and optional `<name>.settings.json`) to load.     |
| `CLAUDE_SESSION_REAL_CLAUDE`      | (auto-discovered)                                             | Absolute path to the real `claude` binary. Overrides discovery; useful in devcontainers or CI.              |
| `CLAUDE_SESSION_OAUTH_CMD`        | (unset)                                                       | Command whose stdout becomes `CLAUDE_CODE_OAUTH_TOKEN`, unless that var is already set. Run under a 5-second timeout. |
| `CLAUDE_SESSION_POST_EXIT_CMD`    | (unset)                                                       | Command run after `claude` exits. Receives `CLAUDE_SESSION_DIR` and `CLAUDE_SESSION_PROFILE` in its env. Failures are logged, not fatal. |
| `CLAUDE_SESSION_SYNC_FILES`       | `.credentials.json`                                           | Colon-separated list of files the upstream CLI may atomic-rewrite — copied in at start, synced back on exit under `flock`. |
| `CLAUDE_SESSION_LINK_FILES`       | `settings.local.json:keybindings.json:CLAUDE.md`              | Colon-separated list of files safe to symlink into the session dir.                                         |
| `CLAUDE_SESSION_LINK_DIRS`        | `skills:agents:rules:commands:hooks`                          | Colon-separated list of directories to symlink into the session dir.                                        |
| `CLAUDE_SESSION_VERBOSE`          | `0`                                                           | `1` enables stderr debug logging (includes hook stderr, resolved paths, cache-hit reporting).               |

The CLI also **forwards** any `CLAUDE_CODE_*` and `ANTHROPIC_*` env vars
unchanged to the child process. That is how profile files inject
`CLAUDE_CODE_USE_VERTEX`, `ANTHROPIC_VERTEX_PROJECT_ID`, etc.

## Secrets discipline

- **Never pass secrets as flags.** Flags are visible in `ps` output and
  shell history. Use env vars or the config file instead.
- **`chmod 600`** on your `config.env` and any `profiles/*.env` file
  that contains a secret-store command or a token. `claude-session
  doctor` warns if your config is world-readable.
- The `CLAUDE_SESSION_OAUTH_CMD` pattern — delegating token lookup to a
  user-provided command — is there so the token never lands on disk in
  this CLI's files. Pair it with `pass`, `gopass`, `bw`, `op`, your
  cloud secret manager, etc.
- `claude-session profile show <name>` redacts values of any variable
  whose name matches `*_TOKEN`, `*_SECRET`, `*_KEY`, `*_PASSWORD`
  unless `--verbose` is passed.

## Missing-config behavior

- No `config.env` at all: treated as if the file is empty. Defaults
  apply. `doctor` notes "no config file found" and points at the
  expected path.
- Missing `profiles/<name>.env` when an explicit profile was requested:
  exit code 6 (profile not found) with a three-part error that lists
  the profiles it did find and the search path it used. Example:

  ```
  Error: profile "vertex" not found.

  What went wrong:
    No file at /home/user/.config/claude-session/profiles/vertex.env
    and no overlay at /home/user/.claude/settings.vertex.json.

  How to fix:
    List available profiles:  claude-session profile list
    Create the profile:       $EDITOR /home/user/.config/claude-session/profiles/vertex.env

  Next:
    claude-session profile list
  ```

- Invalid dotenv syntax: exit code 3 with the offending line number.
- `CLAUDE_SESSION_OAUTH_CMD` / `CLAUDE_SESSION_POST_EXIT_CMD` exit
  non-zero: logged to stderr as a warning. Non-fatal for OAuth (token
  just isn't set; user may already have it in env). Fatal (exit 7)
  for `run` if the pre-start OAuth step fails **and** no token is
  available from any source.

## Migration from single-script predecessors

If you are coming from a single-binary wrapper that hard-coded an
OAuth-token source, a cost-sync tool, or a named Vertex AI profile,
reproduce the old behavior by wiring a hook command and a named
profile:

```
# Wire your own token-lookup command (pass, gopass, bw, op, etc.):
CLAUDE_SESSION_OAUTH_CMD=<your-token-lookup-cmd>

# Wire your own post-exit sync / analytics / cleanup:
CLAUDE_SESSION_POST_EXIT_CMD=<your-post-exit-cmd>
```

And drop a profile file for the backend you want to switch into:

```
# ~/.config/claude-session/profiles/vertex.env
CLAUDE_CODE_USE_VERTEX=1
CLOUD_ML_REGION=<your-region>
ANTHROPIC_VERTEX_PROJECT_ID=<your-gcp-project-id>
```

Activate with `--profile vertex` (or `CLAUDE_SESSION_PROFILE=vertex` in
your shell rc).

Invoke with `claude-session --profile vertex run …` (or set
`CLAUDE_SESSION_PROFILE=vertex` in your shell rc).

## Worked example: minimal to extensive

**Barebones** — no `config.env`, no profile. `claude-session` uses the
`default` profile (which does nothing) and behaves like a plain
isolation wrapper.

**One line** — `config.env` containing `CLAUDE_SESSION_PROFILE=work`,
and `profiles/work.env` containing `ANTHROPIC_MODEL=claude-opus-4-7`.
Now every invocation uses that model.

**Full** — `config.env` sets `CLAUDE_SESSION_OAUTH_CMD` and
`CLAUDE_SESSION_POST_EXIT_CMD`; `profiles/default.env` contains shared
defaults; `profiles/{work,experiment,vertex}.env` contain per-profile
overrides; `profiles/vertex.settings.json` adds a Claude Code-level
overlay for the Vertex profile (for example, different permission
presets). Switch profiles with `--profile <name>` or by setting
`CLAUDE_SESSION_PROFILE` in a directory-scoped env file.
