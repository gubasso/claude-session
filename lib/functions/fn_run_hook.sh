# shellcheck shell=bash
: 'desc: Run a user-configured hook command and echo its stdout.'

cs::fn::run_hook() {
  local cmd=$1
  [[ -n "$cmd" ]] || return 0
  local output=""
  local err=""
  local status=0
  err=$(mktemp)
  output=$(bash -c "$cmd" 2>"$err") || status=$?
  if [[ $status -ne 0 ]]; then
    if [[ "${CLAUDE_SESSION_VERBOSE:-0}" == "1" ]]; then
      sed 's/^/[claude-session] hook stderr: /' "$err" >&2 || true
    else
      cs::helpers::log "warning: hook command failed: $cmd"
    fi
    rm -f "$err"
    return "$status"
  fi
  rm -f "$err"
  printf '%s\n' "$output"
}
