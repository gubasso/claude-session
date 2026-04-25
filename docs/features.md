# Features

The v1 feature inventory, organized by provenance: features **ported**
verbatim (semantically) from the source script, features **generalized**
(the source hard-coded a value; v1 makes it configurable), and features
**added** from the internal agent-CLI design reference. A final section
lists what is explicitly **deferred** beyond v1.

## Ported from the source wrapper

Eight core behaviors carry over unchanged in intent; see
[architecture.md](architecture.md) §"Session-dir lifecycle" for the
step-by-step.

1. **Per-terminal `CLAUDE_CONFIG_DIR` isolation**, keyed by the caller's
   `tty` path (`pts/0` → `pts-0`), with PID fallback when no tty is
   attached. Each terminal gets an independent history, todos, projects,
   and plans.
2. **Secure session directory selection**: project root is
   `$XDG_RUNTIME_DIR/claude-session/` when available, falling back to
   `${XDG_STATE_HOME:-$HOME/.local/state}/claude-session/` — both
   XDG-standard locations, with the spec default hardcoded when the
   env var is unset. No `/tmp` fallback, no wrapper-specific override
   vars. If neither XDG path is usable the wrapper fails fast with
   exit code 5. Strict validation rejects symlinks, non-directories,
   and wrong ownership; mode is forced to 700. Per-terminal sessions
   land under a `sessions/` subdir of that root.
3. **Profile-based `settings.json` overlay** via `jq -s '.[0] * .[1]'`,
   with mtime-based cache invalidation so repeat invocations are fast
   when nothing changed.
4. **File classification** — three modes (`sync` / `link` / `dir-link`)
   for files the wrapper stages into the session directory. The `sync`
   mode's copy + atomic `flock`ed temp-rename pattern is preserved as
   it was written in the source.
5. **Real-binary discovery** prefers the native install path at
   `$HOME/.local/share/claude/versions/` (highest `sort -V`), falling
   back to a `PATH` scan that skips entries resolving back to this
   wrapper. Recursion detection is fatal with remediation.
6. **Session metadata** written as `session-meta.json` (schema v1,
   `profile`, `terminal_id`, `started_at`, `cwd`) so post-exit hooks
   have durable context.
7. **Trap-based exit sync** under `flock`: `sync`-classified files are
   atomically copied back to the shared dir on EXIT, skipping when the
   shared copy is newer (another terminal wrote more recently).
8. **Child-process execution** of the real `claude` binary (not `exec`)
   so the EXIT trap runs and post-exit hooks fire.

## Generalized (was hard-coded; now user-settable)

The source script baked in values that are specific to a single user's
environment. In v1 every one of them is exposed as a config / env
variable so the repo can ship publicly with zero personal data.

| Concept                                           | v1 surface                                                            |
|---------------------------------------------------|-----------------------------------------------------------------------|
| Switch backend (e.g. Vertex AI vs default)        | User-defined profiles under `~/.config/claude-session/profiles/`, selected via `--profile` / `CLAUDE_SESSION_PROFILE`. |
| Cloud project ID                                  | User sets `ANTHROPIC_VERTEX_PROJECT_ID` (or equivalent) in their own `profiles/<name>.env`. |
| Cloud region                                      | User sets `CLOUD_ML_REGION` (or equivalent) in their own `profiles/<name>.env`. |
| Profile naming                                    | Any name; the built-in default is literally `default`. Examples in docs use generic names: `default`, `work`, `experiment`, `vertex`. |
| Pre-startup OAuth-token lookup                    | Any command, via `CLAUDE_SESSION_OAUTH_CMD`. Unset → step skipped.    |
| Post-exit cost-sync / analytics / cleanup         | Any command, via `CLAUDE_SESSION_POST_EXIT_CMD`. Unset → step skipped.|
| Set of files to copy-sync vs symlink              | Overridable via `CLAUDE_SESSION_SYNC_FILES`, `CLAUDE_SESSION_LINK_FILES`, `CLAUDE_SESSION_LINK_DIRS`. |

Migration path for users of the original script is documented in
[config.md](config.md) §"Migration note".

## New in v1 (agent-CLI surface)

The source script is a single opaque binary — either it works or it
fails silently. v1 adds the minimum set of CLI ergonomics from the
internal agent-friendly-CLI design reference, so the tool is usable by
both humans and LLM coding agents.

- **`doctor` subcommand**: structured health check of config file
  presence, shared dir accessibility, real-binary discovery, profile
  validity, and hook invocability. Agents run this first when things
  go sideways.
- **`usage` subcommand**: dumps the entire command tree (flags,
  defaults, examples) in one call — agents pipe it into context once
  and avoid walking `--help` per subcommand.
- **`config show | path | edit`**: inspect the active config,
  resolve the config-file path (honoring
  `CLAUDE_SESSION_CONFIG_DIR`), or open it in `$EDITOR`.
- **`profile list | show <name>`**: enumerate profiles or dump one
  profile's resolved env-var set (secrets redacted unless
  `--verbose`).
- **`session list | clean`**: list active / stale session
  directories, clean with `--older-than <duration>` and `--dry-run`
  / `--yes` guards.
- **Per-subcommand `--help`** with USAGE / DESCRIPTION / FLAGS /
  EXAMPLES / EXIT CODES / SEE ALSO sections, per the CLI-design
  reference. Source of truth for the command contract.
- **Three-part error shape** on every failure path: `What went
  wrong:` / `How to fix:` / `Next:`. Template in
  [architecture.md](architecture.md) §"Error shape".
- **Stable exit codes** (contract for agents): 0 success, 1 generic,
  2 usage, 3 config, 4 binary-not-found, 5 secure-dir-failure, 6
  profile-not-found, 7 hook-failure, 130/143 for SIGINT/SIGTERM.
  Full table in [commands.md](commands.md) §"Exit codes".
- **stdout vs stderr discipline**: data on stdout, everything else on
  stderr. No ANSI on non-tty output.
- **`--dry-run` on destructive operations** (`session clean`).
- **No interactive prompts** when stdin is not a tty — fail fast with
  remediation instead.

## Deferred to v1.1+

These are planned but explicitly out of scope for v1 to keep the first
release shippable.

- `--json` on every read command (full JSON contract, versioned schema).
- `schema <type>` subcommand — print JSON Schema for `session`,
  `profile`, and the error envelope.
- `verify` / `validate` loops for agent use (dry-run → verify → commit
  pattern).
- Cross-agent `SKILL.md` package (lives in a separate repo; symlinked
  into `~/.claude/skills/` and `.agents/skills/`).
- MCP shim over the CLI (only if someone actually needs it; the CLI
  surface is intentionally the primary contract).
- Homebrew / AUR / Nix packaging. v1 ships multi-file + `install.sh`
  via `just install` only.
- `systemd --user` timer for automated stale-session cleanup.
