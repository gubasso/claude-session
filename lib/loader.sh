# shellcheck shell=bash
: 'desc: Dispatch a subcommand by sourcing the matching command file.'

cs::loader::is_command() {
  local cmd=$1
  case "$cmd" in
    run | doctor | usage | config | profile | session) return 0 ;;
    *) return 1 ;;
  esac
}

cs::loader::dispatch() {
  local sub=${1:-run}
  if [[ $# -gt 0 ]]; then
    shift
  fi

  if ! cs::loader::is_command "$sub"; then
    cs::helpers::die 2 "unknown subcommand \"$sub\"." "No claude-session subcommand named \"$sub\" exists." "  See all commands:  claude-session usage" "claude-session usage"
  fi

  local path="$LIB_DIR/commands/cmd_${sub}.sh"
  # shellcheck source=/dev/null
  . "$path"
  "cs::cmd::${sub}" "$@"
}
