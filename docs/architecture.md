# Architecture

This document describes the internal design of `claude-session`: directory
layout, function-namespacing convention, startup flow, and the session-dir
lifecycle. It is the spec that `lib/` source code must conform to.

## Directory layout

```
claude-session/
├── bin/
│   └── claude-session               # thin entry shim: loader → cs::main
├── lib/
│   ├── loader.sh                    # source-on-dispatch
│   ├── core.sh                      # cs::main, global flags, dispatch
│   ├── helpers.sh                   # cs::helpers::* (error shape, log, die, require)
│   ├── commands/                    # one public cs::cmd::<n> per file
│   │   ├── cmd_run.sh               # default: wrap the real claude binary
│   │   ├── cmd_doctor.sh
│   │   ├── cmd_usage.sh
│   │   ├── cmd_config.sh            # show | path | edit
│   │   ├── cmd_profile.sh           # list | show
│   │   └── cmd_session.sh           # list | clean
│   └── functions/                   # one public cs::fn::<n> per file
│       ├── fn_load_config.sh
│       ├── fn_resolve_profile.sh    # manifest-aware resolver, or stock mode
│       ├── fn_compose_profile.sh    # compose layered settings + sidecar
│       ├── fn_apply_profile_env.sh  # export merged env from compose sidecar
│       ├── fn_real_claude.sh        # discover the real claude binary
│       ├── fn_session_dir.sh        # secure session-dir selection
│       ├── fn_terminal_id.sh        # tty-based id with PID fallback
│       ├── fn_sync_files.sh         # copy-based sync (atomic temp+rename)
│       ├── fn_link_files.sh         # symlink files / dirs
│       ├── fn_run_hook.sh           # run user-supplied hook command
│       ├── fn_session_inventory.sh  # walk sessions/, classify active/stale, render table
│       └── fn_error.sh              # three-part error emitter
├── completions/
│   └── claude-session.bash
├── test/
│   ├── test_helper.bash             # common setup
│   ├── cmd_<name>_test.bats         # per-command tests
│   └── fn_<name>_test.bats          # per-function tests
├── .shellcheckrc
├── .pre-commit-config.yaml
├── .editorconfig
├── justfile
├── install.sh                       # invoked by `just install`
└── uninstall.sh                     # invoked by `just uninstall`
```

Test filename convention is `<module>_test.bats` (suffix, not prefix) to
match the convention used across the repository owner's other bash CLIs.

## Namespace convention

| Prefix                | Role                                               |
|-----------------------|----------------------------------------------------|
| `cs::cmd::<n>`        | Subcommand entry point. One per `lib/commands/cmd_<n>.sh`. |
| `cs::fn::<n>`         | Reusable public helper. One per `lib/functions/fn_<n>.sh`. |
| `cs::helpers::<n>`    | Shared low-level utilities defined in `lib/helpers.sh`.    |
| `__<n>`               | Private helper, scoped to the file that defines it. Never called cross-file. |

Every `.sh` file under `lib/` starts with:

```bash
# shellcheck shell=bash
: 'desc: <one-line description of what this file defines>'
```

- No shebang under `lib/` — these files are sourced, never executed.
- The `desc:` sentinel is harvested by help/man generators.

## Entry flow

1. `bin/claude-session` is the only shebanged file. It:
   - sets strict mode (`set -euo pipefail; shopt -s inherit_errexit`),
   - resolves its own path through symlinks (so `stow` / `ln -s` in
     `~/.local/bin` work),
   - computes `SCRIPT_DIR` and `LIB_DIR`,
   - sources `lib/helpers.sh`, `lib/loader.sh`, `lib/core.sh` (in that
     order),
   - calls `cs::main "$@"`.
2. `cs::main` (in `lib/core.sh`):
   - parses global flags: `--profile <name>`, `--config <path>`,
     `--verbose`, `--dry-run`, `--help`, `--version`,
   - loads config via `cs::fn::load_config` (precedence: CLI flag >
     env > file),
   - resolves the active profile via `cs::fn::resolve_profile`
     (manifest mode or stock mode),
   - dispatches via `cs::loader::dispatch`.
3. `cs::loader::dispatch` (in `lib/loader.sh`):
   - takes `<subcommand> [args...]`,
   - sources `${LIB_DIR}/commands/cmd_<subcommand>.sh` on demand,
   - calls `cs::cmd::<subcommand> "$@"`,
   - emits a three-part error with suggestions if the command is unknown.
4. **Bare invocation** (`claude-session` with no subcommand) dispatches to
   `cs::cmd::run`, forwarding any remaining args to the real `claude`
   binary. A `--` separator between wrapper flags and `claude` args is
   recommended for clarity but not required.

Startup is O(1) regardless of how many subcommands exist: only the file
for the actual dispatched command is sourced.

## Session-dir lifecycle (the `run` subcommand)

This is the core behavior — the reason `claude-session` exists. All
values that would be organization-specific are exposed as config /
env variables (see [config.md](config.md)).

1. **Terminal ID** (`cs::fn::terminal_id`):
   - Read `$(tty)`. Strip `/dev/`. Replace `/` with `-` → e.g. `pts-0`,
     `tty1`.
   - If no tty (non-interactive caller), fall back to `pid-$$`.

2. **Secure base directory** (`cs::fn::session_dir`):
   - Project root, in order of preference:
     1. `$XDG_RUNTIME_DIR/claude-session/` — preferred. Systemd-managed,
        ephemeral (cleared on logout), mode 700 by default. The XDG
        spec defines no default for `XDG_RUNTIME_DIR`, so if it is
        unset we move on rather than guess a path.
     2. `${XDG_STATE_HOME:-$HOME/.local/state}/claude-session/` —
        XDG-standard durable fallback. Used when `$XDG_RUNTIME_DIR` is
        unset or fails validation (containers without `pam_systemd`,
        restricted CI, etc.). The spec default
        (`$HOME/.local/state`) is hardcoded as the fallback when
        `XDG_STATE_HOME` itself is unset; no wrapper-specific override
        env var is exposed.
   - No other fallback. No `/tmp`. If neither XDG path resolves, fail
     with exit code 5.
   - Both candidates are validated identically: must be a real
     directory (not a symlink), owned by the current user. Mode is
     forced to 700 before use.
   - Sessions live under a `sessions/` subdir of the project root, so
     sibling state (e.g. `install-manifest` in the state-home case,
     future logs/caches) can coexist without colliding.
   - Final session dir: `<root>/sessions/<terminal-id>` (mode 700).
   - The resolved root and which source picked it (`runtime` vs
     `state`) are recorded in `session-meta.json` and surfaced by
     `claude-session doctor` so a fallback is never silent.

3. **File classification**. Four handling modes for files/dirs the
   upstream `~/.claude/` tree contains:

   | Mode        | Example                                            | Why                                                                             |
   |-------------|----------------------------------------------------|---------------------------------------------------------------------------------|
   | `sync`      | `.credentials.json`, `mcp-needs-auth-cache.json`   | Upstream rewrites atomically (temp + rename), which would break a symlink. Copy in at start, copy back on exit under `flock`. |
   | `link`      | `settings.local.json`, `keybindings.json`, `CLAUDE.md` | Read-only or edited in place. Safe to symlink into the session dir.             |
   | `home-link` | `.claude.json`                                     | Trust/onboarding state that vanilla `claude` also reads. Symlink to `$HOME/<file>` so wrapped and vanilla invocations share one truth. Seeded as `{}` (mode 600) when absent. |
   | `dir-link`  | `skills/`, `agents/`, `rules/`, `commands/`, `hooks/`, `plugins/`  | Shared directories. Symlink the dir; rebuild the symlink if a previous run replaced it with a real dir. |

   All four lists are overridable via env vars documented in
   `docs/config.md`:
   `CLAUDE_SESSION_SYNC_FILES`, `CLAUDE_SESSION_LINK_FILES`,
   `CLAUDE_SESSION_HOME_LINK_FILES`, `CLAUDE_SESSION_LINK_DIRS`
   (colon-separated).

4. **Profile composition** (`cs::fn::compose_profile`):
   - If `cs::fn::resolve_profile` found `profiles/<name>.yaml`, read its
     ordered `settings-layers` list and resolve each layer to
     `$XDG_CONFIG_HOME/claude-session/settings/<layer>.json`.
   - When `$XDG_CACHE_HOME/claude-session/settings.json` exists and is a
     JSON object, prepend it as the lowest-precedence layer. Corrupt
     cache JSON logs a warning and is skipped.
   - Compose the layers in order with `jq -s`; later layers override
     earlier layers.
   - Write `<session-dir>/settings.json` and
     `<session-dir>/.claude-session-compose.json`.
   - Read the merged `.env` block from the compose sidecar and export
     it with `cs::fn::apply_profile_env` before consuming any
     `CLAUDE_SESSION_*` runtime knobs.

5. **Stock mode**:
   - Trigger: implicit selection and no `profiles/default.yaml`.
   - No `settings.json` is written.
   - No profile `env` is applied.
   - Session isolation, sync/link, metadata, and hooks still run.

6. **OAuth hook** (`cs::fn::run_hook` with `CLAUDE_SESSION_OAUTH_CMD`):
   - Only runs if `CLAUDE_CODE_OAUTH_TOKEN` is not already set in the
     environment.
   - Runs the user-supplied command under a 5-second timeout.
   - On success (non-empty stdout, non-whitespace), exports
     `CLAUDE_CODE_OAUTH_TOKEN=<stdout>`.
   - On failure, logs a warning to stderr (unless `--verbose`, in which
     case it logs the full stderr of the hook) and proceeds.

7. **Session metadata**:
   - Write `<session-dir>/session-meta.json`, schema v1, containing:
     `schema`, `profile`, `terminal_id`, `started_at` (ISO 8601 UTC),
     `cwd`. Used by post-exit hooks.

8. **Export** `CLAUDE_CONFIG_DIR=<session-dir>`.

8b. **Auto-trust the current working directory.** When
    `CLAUDE_SESSION_AUTO_TRUST_CWD=1` (default), set
    `projects["$PWD"].hasTrustDialogAccepted=true` and
    `projects["$PWD"].hasCompletedProjectOnboarding=true` in
    `$HOME/.claude.json` (atomic temp + rename, under the
    `$shared_dir/.claude-session.lock`). Skipped when `--dry-run` is set.
    Disable per-session with `CLAUDE_SESSION_AUTO_TRUST_CWD=0`.

9. **Run** the real `claude` binary as a **child process, not via
   `exec`** so that the EXIT trap fires. The child inherits the
   terminal (stdin/stdout/stderr/signals work as expected).

10. **On exit** (trap on `EXIT INT TERM`):
   - `cs::fn::sync_files` copies each `sync`-classified file back to
     the shared dir under `flock` (atomic temp + rename). Skip if the
     session copy is older than the shared copy (another terminal
     wrote more recently).
   - `.claude.json` is **not** in the sync set: it lives as a live symlink
     to `$HOME/.claude.json` (see "File classification"), so trust and
     onboarding state are visible to both wrapped and vanilla `claude` in
     real time without an EXIT-trap merge. The `flock` over
     `$shared_dir/.claude-session.lock` continues to serialize the
     wrapper's own writes to that file (sync-out for credentials, and the
     auto-trust seed in step 8b).
   - After the `sync_files` loop and still inside the same `flock`,
     persist `$session_dir/settings.json` to
     `$XDG_CACHE_HOME/claude-session/settings.json` with atomic
     `jq 'del(.effortLevel, .model, .outputStyle)'` + `mv -f`. Stock mode no-ops because
     `$session_dir/settings.json` does not exist.
   - Run `CLAUDE_SESSION_POST_EXIT_CMD` if set; the command gets
     `CLAUDE_SESSION_DIR` and `CLAUDE_SESSION_PROFILE` in its env.
     Hook failures do **not** change the wrapper's exit code (logged
     only, unless `--verbose`).
   - Exit with the real `claude` binary's status.

11. **Signal mapping** (per bash-CLI convention): SIGINT → exit 130,
    SIGTERM → exit 143.

## Array merge semantics

Layer composition uses jq object multiplication. Objects deep-merge; arrays are replaced wholesale. If a later layer redefines `hooks.Stop` or `permissions.deny`, the later array replaces the earlier one.

## OAuth and `--bare`

OAuth-hook setup, the token-precedence chain, secret-store recipes, the
`--bare` caveat, and per-profile disable patterns are documented in
[auth.md](auth.md). The hook itself runs at step 6 of the lifecycle
above (`cs::fn::run_hook` with `CLAUDE_SESSION_OAUTH_CMD`).

References: upstream CLI reference and Authentication pages on
`code.claude.com`; issue #36852 (`--bare` flag missing from docs);
issue #27900 (interactive mode ignores `ANTHROPIC_API_KEY`).

## Shared session-inventory rendering (`cs::fn::session_inventory`)

`session list`, `session clean`, and `doctor` all surface the same
sessions table (resolved parent + per-terminal rows). That rendering
lives in **one** function — `cs::fn::session_inventory` — and is
called from each of those subcommands. The function:

1. Resolves the sessions parent (delegates to `cs::fn::session_dir`)
   and reports which XDG source picked it (`runtime` / `state`).
2. Walks `<root>/sessions/`, classifies each entry as active (owning
   tty present) or stale, and collects size + mtime.
3. Emits a deterministic table on stdout with optional flags
   (`--no-header`, `--absolute`) honored uniformly across callers.

This is the single source of truth for the "common ancestor at top,
leaf rows below" shape. Adding a column or changing the sort order
is a one-file edit; `doctor`'s overlap with `session list` is by
construction, not by duplication.

## Real-binary discovery (`cs::fn::real_claude`)

Look up the path of the real `claude` binary, avoiding wrapper recursion.

1. If `CLAUDE_SESSION_REAL_CLAUDE` is set and executable, use it.
2. Else if `$HOME/.local/share/claude/versions/` exists (native install
   layout), pick the highest-sorting entry (`sort -V | tail -n1`) that
   is executable and does **not** resolve back to this wrapper.
3. Else scan `PATH` entry by entry, skipping anything that resolves to
   this wrapper.

If none found, or if the candidate resolves back to this wrapper, emit a
three-part error (exit code 4) with concrete remediation:
`How to fix:` points at `claude login`, the native install path, and
the `CLAUDE_SESSION_REAL_CLAUDE` override.

## Strict mode

```bash
set -euo pipefail
shopt -s inherit_errexit failglob nullglob
```

Known limits:

- `set -e` is disabled inside `$(...)`, `if`/`&&`/`||` chains, and
  `local var=$(...)`. For load-bearing logic prefer explicit
  `|| cs::helpers::die` over implicit `set -e`.
- Do **not** set `IFS=$'\n\t'` globally — quote everything and use
  arrays instead.
- `inherit_errexit` is guarded for bash < 4.4:
  `shopt -s inherit_errexit 2>/dev/null || true`.

Full rationale and references live in `docs/development.md` §"Strict
mode policy".

## stdout vs stderr

- **stdout**: parseable data only (JSON when `--json` is implemented
  later; stable text otherwise).
- **stderr**: progress, warnings, logs, error messages including the
  three-part error shape.
- No ANSI escape codes on either stream unless `[[ -t 1 ]]` is true.
- Use `printf '%s\n' ...` over `echo` (portable across bash versions).

## Error shape

Every failure path emits a three-part error on **stderr**, always
paired with a non-zero exit code. Template:

```
Error: <one-line summary>

What went wrong:
  <1–3 lines of detail>

How to fix:
  <concrete command 1>
  <concrete command 2>

Next:
  <optional follow-up hint>
```

The template is provided by `cs::fn::error` / `cs::helpers::die`.
Agents rely on this shape to self-correct without prompting the user —
don't skip parts or invent new ones.

Exit code table (stable contract; agents may depend on it) lives in
`docs/commands.md` §"Exit codes".

## Temp files and signals

Any session-dir creation pairs with a trap:

```bash
trap 'cs::fn::sync_files; cs::fn::run_hook "$CLAUDE_SESSION_POST_EXIT_CMD"' EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
```

- Use single-quoted trap bodies (expansion happens at trap time, not
  registration time).
- `mktemp -d` for ephemeral scratch, always paired with a cleanup trap.

## Data flow summary

```
  user invocation
        │
  bin/claude-session   ── sources ──▶ lib/{helpers,loader,core}.sh
        │
  cs::main             ── dispatches to ──▶ cs::cmd::<sub>
        │
  cs::cmd::run         ── uses ──▶ cs::fn::{terminal_id, session_dir,
        │                                    compose_profile, apply_profile_env,
        │                                    link_files, real_claude,
        │                                    run_hook}
        │
  exec child: real claude
        │
  EXIT trap            ── sync_files → run_hook(POST_EXIT_CMD) → exit child status
```
