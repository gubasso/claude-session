# shellcheck shell=bash
: 'desc: Wrap the real claude binary with per-terminal session isolation.'

__run_help() {
  cat <<'EOF'
USAGE:
  claude-session run [--profile <name>] [--dry-run] [-- <claude_args>...]
  claude-session [<claude_args>...]
EOF
}

__run_write_meta() {
  local file=$1
  local profile=$2
  local terminal_id=$3
  local root=$4
  local source=$5
  local cwd=$6
  local started_at
  started_at=$(date -u '+%FT%TZ')
  local tmp="$file.tmp.$$"
  {
    printf '{\n'
    printf '  "schema": 1,\n'
    printf '  "profile": "%s",\n' "$(cs::helpers::json_escape "$profile")"
    printf '  "terminal_id": "%s",\n' "$(cs::helpers::json_escape "$terminal_id")"
    printf '  "started_at": "%s",\n' "$started_at"
    printf '  "cwd": "%s",\n' "$(cs::helpers::json_escape "$cwd")"
    printf '  "session_root": "%s",\n' "$(cs::helpers::json_escape "$root")"
    printf '  "session_root_source": "%s"\n' "$(cs::helpers::json_escape "$source")"
    printf '}\n'
  } >"$tmp"
  mv -f "$tmp" "$file"
}

__cs_run_exit_trap() {
  local fallback=$?
  local status
  if [[ -n "${__CS_RUN_FORCED_STATUS:-}" ]]; then
    status=$__CS_RUN_FORCED_STATUS
  elif [[ -n "${__CS_RUN_CHILD_STATUS_SET:-}" ]]; then
    status=$__CS_RUN_CHILD_STATUS
  else
    status=$fallback
  fi
  trap - EXIT
  if [[ -n "${__CS_RUN_SESSION_DIR:-}" && -n "${__CS_RUN_SHARED_DIR:-}" ]]; then
    cs::helpers::source_fn sync_files
    cs::fn::sync_files out "$__CS_RUN_SESSION_DIR" "$__CS_RUN_SHARED_DIR" || true
  fi
  if [[ -n "${CLAUDE_SESSION_POST_EXIT_CMD:-}" ]]; then
    cs::helpers::source_fn run_hook
    CLAUDE_SESSION_DIR=${__CS_RUN_SESSION_DIR:-}
    export CLAUDE_SESSION_DIR CLAUDE_SESSION_PROFILE
    cs::fn::run_hook "$CLAUDE_SESSION_POST_EXIT_CMD" >/dev/null || true
  fi
  exit "$status"
}

cs::cmd::run() {
  local dry_run=${CLAUDE_SESSION_DRY_RUN:-0}
  local -a pass_args=()
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --profile)
        [[ -n "${2:-}" ]] || cs::helpers::die 2 "missing value for --profile." "--profile requires a profile name." "  claude-session run --profile default"
        CS_CLI_PROFILE=$2
        CLAUDE_SESSION_PROFILE=$2
        export CS_CLI_PROFILE CLAUDE_SESSION_PROFILE
        cs::helpers::source_fn resolve_profile
        cs::fn::resolve_profile
        shift 2
        ;;
      --dry-run)
        dry_run=1
        shift
        ;;
      --help | -h)
        __run_help
        return 0
        ;;
      --)
        shift
        pass_args=("$@")
        break
        ;;
      *)
        pass_args+=("$1")
        shift
        ;;
    esac
  done

  local bare=0
  local arg=""
  for arg in "${pass_args[@]}"; do
    [[ "$arg" == "--bare" ]] && bare=1
  done

  local shared_dir=${CLAUDE_SESSION_SHARED_DIR:-$HOME/.claude}
  mkdir -p "$shared_dir"

  cs::helpers::source_fn terminal_id
  cs::helpers::source_fn session_dir
  cs::helpers::source_fn sync_files
  cs::helpers::source_fn link_files
  cs::helpers::source_fn merge_settings
  cs::helpers::source_fn real_claude
  cs::helpers::source_fn run_hook

  local root
  root=$(cs::fn::session_dir)
  local root_source
  root_source=$(cs::fn::session_root_source_for "$root")
  local terminal_id
  terminal_id=$(cs::fn::terminal_id)
  local session_dir="$root/sessions/$terminal_id"
  mkdir -p "$session_dir"
  chmod 700 "$session_dir"

  cs::fn::sync_files in "$session_dir" "$shared_dir"
  cs::fn::link_files "$session_dir" "$shared_dir"
  cs::fn::merge_settings "$shared_dir" "${CLAUDE_SESSION_PROFILE:-default}" "$session_dir" "${CLAUDE_SESSION_CONFIG_DIR:-$(cs::helpers::config_dir_default)}"
  __run_write_meta "$session_dir/session-meta.json" "${CLAUDE_SESSION_PROFILE:-default}" "$terminal_id" "$root" "$root_source" "$PWD"

  local real_claude
  real_claude=$(cs::fn::real_claude)

  if [[ $bare -eq 0 && -z "${CLAUDE_CODE_OAUTH_TOKEN:-}" && -n "${CLAUDE_SESSION_OAUTH_CMD:-}" ]]; then
    local token=""
    if token=$(cs::fn::run_hook "$CLAUDE_SESSION_OAUTH_CMD" --timeout 5 --fatal); then
      token=${token//$'\n'/}
      token=${token//$'\r'/}
      if [[ -n "${token//[[:space:]]/}" ]]; then
        CLAUDE_CODE_OAUTH_TOKEN=$token
        export CLAUDE_CODE_OAUTH_TOKEN
      fi
    fi
    if [[ -z "${CLAUDE_CODE_OAUTH_TOKEN:-}" ]]; then
      cs::helpers::die 7 "OAuth hook failed." "CLAUDE_SESSION_OAUTH_CMD did not provide a token and CLAUDE_CODE_OAUTH_TOKEN is unset." "  Fix CLAUDE_SESSION_OAUTH_CMD or export CLAUDE_CODE_OAUTH_TOKEN." "claude-session doctor"
    fi
  fi

  export CLAUDE_CONFIG_DIR=$session_dir
  export CLAUDE_SESSION_DIR=$session_dir
  export CLAUDE_SESSION_PROFILE=${CLAUDE_SESSION_PROFILE:-default}

  if [[ $dry_run -eq 1 ]]; then
    printf 'session_dir=%s\n' "$session_dir"
    printf 'settings=%s\n' "$session_dir/settings.json"
    printf 'oauth_hook=%s\n' "$([[ $bare -eq 1 ]] && printf 'skipped (--bare)' || printf '%s' "${CLAUDE_SESSION_OAUTH_CMD:-unset}")"
    printf 'post_exit_hook=%s\n' "${CLAUDE_SESSION_POST_EXIT_CMD:-unset}"
    printf 'real_claude=%s\n' "$real_claude"
    printf 'argv=%s' "$real_claude"
    for arg in "${pass_args[@]}"; do
      printf ' %q' "$arg"
    done
    printf '\n'
    return 0
  fi

  __CS_RUN_SESSION_DIR=$session_dir
  __CS_RUN_SHARED_DIR=$shared_dir
  __CS_RUN_CHILD_STATUS=0
  __CS_RUN_CHILD_STATUS_SET=""
  __CS_RUN_FORCED_STATUS=""
  export __CS_RUN_SESSION_DIR __CS_RUN_SHARED_DIR __CS_RUN_CHILD_STATUS \
    __CS_RUN_CHILD_STATUS_SET __CS_RUN_FORCED_STATUS
  trap '__cs_run_exit_trap' EXIT
  trap '__CS_RUN_FORCED_STATUS=130; exit 130' INT
  trap '__CS_RUN_FORCED_STATUS=143; exit 143' TERM

  "$real_claude" "${pass_args[@]}" && __CS_RUN_CHILD_STATUS=0 || __CS_RUN_CHILD_STATUS=$?
  __CS_RUN_CHILD_STATUS_SET=1
  return "$__CS_RUN_CHILD_STATUS"
}
