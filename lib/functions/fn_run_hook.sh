# shellcheck shell=bash
: 'desc: Run user-configured hook commands with optional timeout handling.'

cs::fn::run_hook() {
  local cmd=$1
  shift || true
  local timeout_s=""
  local fatal=0
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --timeout)
        timeout_s=$2
        shift 2
        ;;
      --fatal)
        fatal=1
        shift
        ;;
      *)
        cs::helpers::die 2 "unknown hook flag \"$1\"." "run_hook accepts --timeout and --fatal only." "  Report this as an internal bug."
        ;;
    esac
  done

  [[ -n "$cmd" ]] || return 0
  local output=""
  local err=""
  local status=0
  err=$(mktemp)
  if [[ -n "$timeout_s" && "$(command -v timeout || true)" != "" ]]; then
    output=$(timeout "$timeout_s" bash -c "$cmd" 2>"$err") || status=$?
  else
    output=$(bash -c "$cmd" 2>"$err") || status=$?
  fi
  if [[ $status -ne 0 ]]; then
    if [[ "${CLAUDE_SESSION_VERBOSE:-0}" == "1" ]]; then
      sed 's/^/[claude-session] hook stderr: /' "$err" >&2 || true
    else
      cs::helpers::log "warning: hook command failed: $cmd"
    fi
    rm -f "$err"
    [[ $fatal -eq 1 ]] && return "$status"
    return 0
  fi
  rm -f "$err"
  printf '%s\n' "$output"
}
