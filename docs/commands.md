# Commands

Every subcommand `claude-session` ships in v1, documented using the same
template as each command's own `--help` output. This file is the
canonical source for the user-facing contract: agents and scripts may
rely on the exit codes, flag names, and output shape documented here.

## Command tree

```
claude-session
├── run [--profile <n>] [--dry-run] [-- <claude_args>...]
├── doctor [--verbose]
├── usage
├── config
│   ├── show
│   ├── path
│   └── edit
├── profile
│   ├── list
│   └── show <name> [--verbose]
└── session
    ├── list
    └── clean [--older-than <duration>] [--dry-run] [--yes]
```

`claude-session` with **no subcommand** dispatches to `run`, forwarding
any remaining arguments to the real `claude` binary. A `--` separator
is recommended but not required.

Example:

```sh
claude-session --profile work -- chat "hello"
```

## Global flags

These are parsed before subcommand dispatch and apply to every command.

| Flag                          | Meaning                                                                  |
|-------------------------------|--------------------------------------------------------------------------|
| `--profile <name>`            | Override `CLAUDE_SESSION_PROFILE` for this invocation.                   |
| `--config <path>`             | Override the config-file path (defaults to `$XDG_CONFIG_HOME/claude-session/config.env`). |
| `--verbose`                   | Enable stderr debug logging. Equivalent to `CLAUDE_SESSION_VERBOSE=1`.   |
| `--dry-run`                   | Where supported by a subcommand, show what would happen without acting.  |
| `--help`, `-h`                | Show help for the invoked command and exit 0.                            |
| `--version`                   | Print version string and exit 0.                                         |

## Exit codes

Stable contract. Agents and scripts may depend on these.

| Code | Meaning                                                                                  |
|------|------------------------------------------------------------------------------------------|
| `0`  | Success.                                                                                 |
| `1`  | Generic error (fallback).                                                                |
| `2`  | Usage error — bad flag, unknown subcommand, missing required argument.                   |
| `3`  | Config missing or invalid (syntax error, unreadable, not found when required).           |
| `4`  | Real `claude` binary not found (or discovery resolved back to this wrapper).             |
| `5`  | Secure session-dir validation failed: neither `XDG_RUNTIME_DIR` nor the `XDG_STATE_HOME` fallback resolved a usable, owner-correct, non-symlink directory. |
| `6`  | Profile not found — name passed via `--profile` / `CLAUDE_SESSION_PROFILE` has no file.  |
| `7`  | Reserved for fatal hook failures. No current command exits 7 (OAuth and post-exit hook failures are warn-only and fall through to native auth / normal exit). |
| `130`| Interrupted by SIGINT (128 + 2).                                                         |
| `143`| Terminated by SIGTERM (128 + 15).                                                        |

Codes 8–127 are reserved for future domain errors. Codes above 128 are
reserved for signal-based exits (POSIX convention).

---

## Per-subcommand reference

### `claude-session run`

The default. Wraps the real `claude` binary with a per-terminal
isolated `CLAUDE_CONFIG_DIR`.

```
USAGE:
  claude-session run [--profile <name>] [--dry-run] [-- <claude_args>...]
  claude-session [<claude_args>...]                 # bare form; `run` is default

DESCRIPTION:
  Create (or reuse) a session directory for the current terminal,
  resolve either a manifest-backed profile or stock mode, compose
  layered settings when a manifest is active, symlink shared config,
  run the optional OAuth hook, and exec the real claude binary as a
  child process. On exit, sync back copy-classified files and run the
  optional post-exit hook. Seed per-project trust into
  $HOME/.claude.json on launch when CLAUDE_SESSION_AUTO_TRUST_CWD=1
  (default).

FLAGS:
  --profile <name>   Active profile. Overrides CLAUDE_SESSION_PROFILE.
  --dry-run          Print the plan (mode, manifest, session dir,
                     settings path, layers, hooks, final claude
                     invocation) and exit 0 without invoking claude.

EXAMPLES:
  claude-session run
  claude-session --profile vertex run
  claude-session run -- chat "summarize today's commits"
  claude-session --dry-run run

EXIT CODES:
  0    claude binary exited successfully
  1    generic error
  3    config invalid
  4    real claude binary not found
  5    secure-dir validation failed
  6    profile not found
  130  SIGINT
  143  SIGTERM
  *    anything else is the real claude's exit code, forwarded unchanged

OUTPUT (success):
  stdout/stderr from the real claude are forwarded unchanged.

OUTPUT (error, example):
  Error: real claude binary not found.

  What went wrong:
    Searched $HOME/.local/share/claude/versions/ and PATH; no executable
    named "claude" that does not resolve to this wrapper.

  How to fix:
    Install via:                claude install
    Or set an override:         CLAUDE_SESSION_REAL_CLAUDE=/path/to/claude

  Next:
    claude-session doctor

SEE ALSO:
  claude-session doctor
  claude-session profile show <name>
  docs/architecture.md §"Session-dir lifecycle"
```

### `claude-session doctor`

Structured health check. Run this first when something goes wrong.

```
USAGE:
  claude-session doctor [--verbose]

DESCRIPTION:
  The verbose debug interface for the wrapper. Bundles every
  health check (config presence and permissions, shared dir
  accessibility, active profile validity, manifest composition,
  real-binary discovery, hook invocability, session-dir resolution)
  AND the full session inventory in one call, so a user or agent has
  a single command to dump for triage.

  Prints one line per check with an OK / WARN / FAIL status, then
  a "Sessions" block — produced by the shared
  `cs::fn::session_inventory` helper, the same renderer
  `session list` uses — keyed by the resolved sessions parent
  (common ancestor surfaced once, per-session rows underneath).
  Closes with a "Next:" section listing remediation for any
  non-OK check.

  WARN is emitted when sessions resolve via the `XDG_STATE_HOME`
  fallback so the downgrade is never silent.

  The session block is intentionally identical to `session list`'s
  output: doctor is the everything-at-once view, `session list` is
  the parseable inventory in isolation. Both call the same renderer
  — there is no duplicated code.

FLAGS:
  --verbose   Include resolved paths and the env vars the active
              profile would set. Redaction rules in docs/config.md
              apply (values of *_TOKEN / *_SECRET / *_KEY /
              *_PASSWORD are printed as "<redacted>").

EXAMPLES:
  claude-session doctor
  CLAUDE_SESSION_PROFILE=vertex claude-session doctor --verbose

EXIT CODES:
  0   all checks passed (or only WARNs, no FAILs)
  1   one or more checks FAILed (details in output)
  3   no config available and required checks cannot run

OUTPUT (success, example — sessions on XDG_RUNTIME_DIR):
  config          OK    /home/user/.config/claude-session/config.env (mode 600)
  profile         OK    "default"
  mode            OK    manifest (/home/user/.config/claude-session/profiles/default.yaml)
  yq              OK    /usr/bin/yq
  shared dir      OK    /home/user/.claude (exists, readable)
  real claude     OK    /home/user/.local/share/claude/versions/1.2.3
  oauth hook      —     CLAUDE_SESSION_OAUTH_CMD unset (skipped)
  post-exit hook  —     CLAUDE_SESSION_POST_EXIT_CMD unset (skipped)
  session root    OK    /run/user/1000/claude-session/  (XDG_RUNTIME_DIR, mode 700)
  jq              OK    /usr/bin/jq
  base64          OK    /usr/bin/base64
  flock           OK    /usr/bin/flock
  timeout         OK    /usr/bin/timeout

  Sessions under /run/user/1000/claude-session/sessions/  (3 dirs: 1 active, 2 stale)
    terminal_id   status   size      mtime
    pts-0         active   412 KiB   2026-04-25T09:02:11Z
    pts-3         stale    311 KiB   2026-04-24T14:17:42Z
    pid-48221     stale    184 KiB   2026-04-23T22:01:09Z

OUTPUT (fallback example — XDG_STATE_HOME):
  ...
  session root    WARN  /home/user/.local/state/claude-session/  (XDG_STATE_HOME fallback; XDG_RUNTIME_DIR unset)

  Sessions under /home/user/.local/state/claude-session/sessions/  (1 dir: 1 active)
    terminal_id   status   size      mtime
    pts-0         active   188 KiB   2026-04-25T09:02:11Z

  Next:
    Restore the runtime dir:  loginctl enable-linger $USER  (or set XDG_RUNTIME_DIR)

SEE ALSO:
  claude-session config show
  claude-session profile list
  claude-session session list
  docs/config.md
```

### `claude-session usage`

Dump the entire command tree (flags, defaults, examples) in one call
for ingestion by agents or for grep-able reference. This is the
aggregate form Linearis / the internal agent-CLI reference recommend.

```
USAGE:
  claude-session usage

DESCRIPTION:
  Print every subcommand's --help output concatenated in a
  deterministic order, preceded by the command tree and the global
  flags table. The output is plain text, no ANSI, stable across
  patch releases. Agents can pipe this into context once and skip
  per-subcommand --help calls.

FLAGS:
  (none)

EXAMPLES:
  claude-session usage
  claude-session usage | grep -A 3 "EXIT CODES"

EXIT CODES:
  0   always (unless invoked with bad global flags, → 2)

OUTPUT:
  ~300–600 lines of plain text, grouped per subcommand.

SEE ALSO:
  claude-session --help
  claude-session <cmd> --help
```

### `claude-session config show`

Dump the effective, resolved config (file + env overrides) to stdout.

```
USAGE:
  claude-session config show [--verbose]

DESCRIPTION:
  Print every CLAUDE_SESSION_* variable and its resolved value, one
  per line as KEY=VALUE. Variables unset with no default appear with
  a "(unset)" placeholder. Redaction (see docs/config.md
  §"Secrets discipline") applies unless --verbose.

FLAGS:
  --verbose   Do not redact secret-like variable values.

EXAMPLES:
  claude-session config show
  claude-session --profile work config show --verbose

EXIT CODES:
  0   success
  3   config file exists but is syntactically invalid

OUTPUT (example):
  CLAUDE_SESSION_CONFIG_DIR=/home/user/.config/claude-session
  CLAUDE_SESSION_SHARED_DIR=/home/user/.claude
  CLAUDE_SESSION_PROFILE=work
  CLAUDE_SESSION_REAL_CLAUDE=(unset, auto-discover)
  CLAUDE_SESSION_OAUTH_CMD=<redacted>
  CLAUDE_SESSION_POST_EXIT_CMD=my-cost-sync --session-dir "$CLAUDE_SESSION_DIR"
  CLAUDE_SESSION_SYNC_FILES=.credentials.json:mcp-needs-auth-cache.json
  CLAUDE_SESSION_LINK_FILES=settings.local.json:keybindings.json:CLAUDE.md
  CLAUDE_SESSION_HOME_LINK_FILES=.claude.json
  CLAUDE_SESSION_LINK_DIRS=skills:agents:rules:commands:hooks:plugins
  CLAUDE_SESSION_AUTO_TRUST_CWD=1
  CLAUDE_SESSION_VERBOSE=0

SEE ALSO:
  claude-session config path
  claude-session config edit
  docs/config.md
```

### `claude-session config path`

Print the resolved path of the main config file. Useful for scripts.

```
USAGE:
  claude-session config path

DESCRIPTION:
  Print the full path to `config.env` that claude-session would load
  given the current environment (honors --config, CLAUDE_SESSION_CONFIG_DIR,
  and XDG_CONFIG_HOME). Always one line, always on stdout. Non-existent
  files are still printed (the path is "what would be loaded"); exit 0.

FLAGS:
  (none)

EXAMPLES:
  claude-session config path
  $EDITOR "$(claude-session config path)"

EXIT CODES:
  0   success

OUTPUT (example):
  /home/user/.config/claude-session/config.env

SEE ALSO:
  claude-session config edit
```

### `claude-session config edit`

Open the main config file in `$EDITOR`, creating it if missing.

```
USAGE:
  claude-session config edit

DESCRIPTION:
  Resolve the config-file path (as `config path` does), create the
  parent directory and file (mode 600) if they do not exist, then
  exec $EDITOR (or $VISUAL, or `vi` as the ultimate fallback) on it.

FLAGS:
  (none)

EXAMPLES:
  claude-session config edit
  VISUAL=nvim claude-session config edit

EXIT CODES:
  0   editor exited normally
  1   editor exited non-zero (forwarded)
  3   parent directory could not be created

OUTPUT:
  Whatever the editor writes to the terminal. No output of our own.

SEE ALSO:
  claude-session config path
  claude-session config show
```

### `claude-session profile list`

List every profile discoverable under `$CLAUDE_SESSION_CONFIG_DIR/profiles/`.

```
USAGE:
  claude-session profile list

DESCRIPTION:
  Enumerate profiles. A profile is any file matching
  profiles/<name>.yaml (trailing .yaml is the marker). Output is one
  profile name per line, sorted alphabetically. The active resolved
  manifest profile (from --profile / CLAUDE_SESSION_PROFILE /
  default.yaml) is marked with a leading "*" on that line. Stock mode
  marks nothing.

FLAGS:
  (none)

EXAMPLES:
  claude-session profile list

EXIT CODES:
  0   success (including zero profiles — stdout is just empty)
  3   profiles directory unreadable

OUTPUT (example):
  * default
    work

SEE ALSO:
  claude-session profile show <name>
  docs/config.md
```

### `claude-session profile show <name>`

Dump one profile's manifest, resolved layers, and merged env vars.

```
USAGE:
  claude-session profile show <name> [--verbose]

DESCRIPTION:
  Compose the profile manifest in an isolated scratch directory and
  print the manifest path, ordered layer paths, and merged KEY=VALUE
  pairs from the composed `.env` block. Secret-like values are
  redacted unless --verbose.

FLAGS:
  --verbose   Do not redact secret-like variable values.

EXAMPLES:
  claude-session profile show default
  claude-session profile show vertex --verbose

EXIT CODES:
  0   success
  3   profile manifest or layer is invalid
  6   profile not found

OUTPUT (example):
  # profile: vertex
  # manifest: /home/user/.config/claude-session/profiles/vertex.yaml
  # layers:
  #   1: /home/user/.config/claude-session/settings/base.json
  #   2: /home/user/.config/claude-session/settings/vertex.json
  CLAUDE_CODE_USE_VERTEX=1
  CLOUD_ML_REGION=us-central1
  ANTHROPIC_VERTEX_PROJECT_ID=<redacted>

SEE ALSO:
  claude-session profile list
  claude-session config show
```

### `claude-session session list`

Inventory active and stale session directories.

```
USAGE:
  claude-session session list

DESCRIPTION:
  Walk the resolved sessions parent
  ($XDG_RUNTIME_DIR/claude-session/sessions/, or the
  $XDG_STATE_HOME/claude-session/sessions/ fallback) for
  per-terminal session directories.

  Output shape: a single header line printing the absolute parent
  path (the common ancestor) and which XDG source resolved it,
  followed by a column header and one row per directory. Each row
  lists terminal_id, status (active if the owning tty still exists,
  stale otherwise), size, and mtime — the path is implicit from the
  header so rows stay short and aligned. Sorted by mtime descending.

FLAGS:
  --absolute       Print fully-qualified paths in the terminal_id
                   column instead of the leaf basename. Useful when
                   piping into `xargs rm -r` or other path-consuming
                   tools.
  --no-header      Suppress the parent-path header and the column
                   header. Emit data rows only — the most stable
                   shape for `awk` / `cut` pipelines.

EXAMPLES:
  claude-session session list
  claude-session session list --no-header | awk '$2 == "stale"'
  claude-session session list --absolute --no-header | awk '$2 == "stale" { print $1 }'

EXIT CODES:
  0   success (zero rows is still success)
  3   base directory unreadable
  5   no usable XDG session root (see `doctor`)

OUTPUT (example, default):
  /run/user/1000/claude-session/sessions/  (XDG_RUNTIME_DIR)
  # terminal_id   status   size     mtime
  pts-0           active   412 KiB  2026-04-24T18:03:11Z
  pts-3           stale    311 KiB  2026-04-24T14:17:42Z
  pid-48221       stale    184 KiB  2026-04-23T22:01:09Z

OUTPUT (example, --no-header):
  pts-0           active   412 KiB  2026-04-24T18:03:11Z
  pts-3           stale    311 KiB  2026-04-24T14:17:42Z
  pid-48221       stale    184 KiB  2026-04-23T22:01:09Z

SEE ALSO:
  claude-session session clean
  claude-session doctor
```

### `claude-session session clean`

Remove stale session directories.

```
USAGE:
  claude-session session clean [--older-than <duration>] [--dry-run] [--yes]

DESCRIPTION:
  Remove session directories under the resolved sessions parent
  ($XDG_RUNTIME_DIR/claude-session/sessions/, or the
  $XDG_STATE_HOME/claude-session/sessions/ fallback) whose status
  is "stale" per `session list`, and — when --older-than is set —
  whose mtime is older than the given duration. Never removes
  active session dirs. Prompts for confirmation unless --yes or
  --dry-run.

  Output mirrors `session list`'s shape: the resolved parent path
  is printed once at the top as the common ancestor, then each
  candidate is listed by terminal_id alone underneath.

FLAGS:
  --older-than <dur>   Only delete directories older than <dur>. Accepts
                       "N[smhdw]" suffix syntax (30s, 15m, 2h, 7d, 4w).
                       Default: no age threshold (any stale dir is a
                       candidate).
  --dry-run            Print what would be removed and exit 0 without
                       deleting anything.
  --yes                Skip the interactive confirmation. Required when
                       stdin is not a TTY (non-interactive usage).

EXAMPLES:
  claude-session session clean --dry-run
  claude-session session clean --older-than 7d --yes
  claude-session session clean --yes

EXIT CODES:
  0   success
  1   one or more directories could not be removed
  2   missing --yes under non-interactive stdin
  3   base directory unreadable
  5   no usable XDG session root (see `doctor`)

OUTPUT (example, --dry-run):
  Would remove 2 stale session directories under
  /run/user/1000/claude-session/sessions/:
    pts-3       (2026-04-24, 311 KiB)
    pid-48221   (2026-04-23, 184 KiB)

  Next:
    Remove them:  claude-session session clean --yes

SEE ALSO:
  claude-session session list
  claude-session doctor
```

## Output shape conventions

- Every command writes parseable data to stdout and logs / progress /
  errors to stderr.
- Success output for mutation commands ends with an optional `Next:`
  block of 2–3 plausible follow-up commands, per the agent-CLI design
  reference.
- Error output (on stderr) follows the three-part shape:
  `What went wrong:` / `How to fix:` / `Next:`. See
  [architecture.md](architecture.md) §"Error shape".
- No ANSI color unless `[[ -t 1 ]]` is true.
- List commands are tabular and grep-friendly; the first line is a
  `# header` comment when columns exist.
