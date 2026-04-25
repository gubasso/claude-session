# shellcheck shell=bash
: 'desc: Emit the stable three-part claude-session error shape.'

cs::fn::error() {
  local summary=$1
  local detail=$2
  local fix=$3
  local next=${4:-}

  printf 'Error: %s\n\n' "$summary" >&2
  printf 'What went wrong:\n  %s\n\n' "$detail" >&2
  printf 'How to fix:\n%s\n' "$fix" >&2
  if [[ -n "$next" ]]; then
    printf '\nNext:\n  %s\n' "$next" >&2
  fi
}
