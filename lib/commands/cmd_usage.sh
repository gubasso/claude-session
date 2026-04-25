# shellcheck shell=bash
: 'desc: Print the full stable claude-session command reference.'

cs::cmd::usage() {
  cat <<EOF
claude-session 0.1.0

USAGE:
  claude-session [--profile <name>] [--config <path>] [--verbose] [--dry-run] <command> [args...]
  claude-session [<claude_args>...]

COMMAND TREE:
  run [--profile <name>] [--dry-run] [-- <claude_args>...]
  doctor [--verbose]
  usage
  config show|path|edit
  profile list|show <name>
  session list|clean

GLOBAL FLAGS:
  --profile <name>  Override CLAUDE_SESSION_PROFILE.
  --config <path>   Override the config-file path.
  --verbose         Enable debug logging.
  --dry-run         Show what would happen where supported.
  --help, -h        Show help.
  --version         Print version.

EXIT CODES:
  0 success
  1 generic error
  2 usage error
  3 config missing or invalid
  4 real claude binary not found
  5 secure session-dir validation failed
  6 profile not found
  7 reserved for fatal hook failures (no current path uses this)
  130 interrupted by SIGINT
  143 terminated by SIGTERM

SUBCOMMANDS:

claude-session run [--profile <name>] [--dry-run] [-- <claude_args>...]
  Wrap the real claude binary with a per-terminal CLAUDE_CONFIG_DIR.

claude-session doctor [--verbose]
  Run config, binary, hook, dependency, and session-root health checks.

claude-session usage
  Print this aggregate command reference.

claude-session config show [--verbose]
  Print effective CLAUDE_SESSION_* config values.

claude-session config path
  Print the resolved config.env path.

claude-session config edit
  Open the config file in \$EDITOR, creating it if missing.

claude-session profile list
  List profiles under the config profiles directory.

claude-session profile show <name> [--verbose]
  Print one profile's env assignments.

claude-session session list [--absolute] [--no-header]
  Inventory active and stale session directories.

claude-session session clean [--older-than <duration>] [--dry-run] [--yes]
  Remove stale session directories.

SEE ALSO:
  docs/commands.md
EOF
}
