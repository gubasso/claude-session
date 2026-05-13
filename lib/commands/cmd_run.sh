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

__run_seed_home_link_files() {
  # Generic locked bootstrap for every file listed in
  # CLAUDE_SESSION_HOME_LINK_FILES. Runs after fn_link_files has
  # created the session-side symlinks and before any in-place writes
  # (auto-trust, Claude Code itself), so the symlink target always
  # exists by the time exec hands off. The flock serializes concurrent
  # pts launches so two first-launch terminals can never both pass an
  # unlocked existence check and overwrite a sibling's content with
  # "{}" (Stage 5 round 1 finding 1, round 2 finding 1).
  local lock_file=$1
  local home_link_files=${CLAUDE_SESSION_HOME_LINK_FILES:-.claude.json}
  command -v flock >/dev/null 2>&1 || cs::helpers::die 3 "flock is required." "Seeding home-linked files needs flock to serialize concurrent launches." "  Install util-linux flock."
  local -a items=()
  cs::helpers::split_colon "$home_link_files" items
  local item
  (
    flock 9
    for item in "${items[@]}"; do
      [[ -n "$item" ]] || continue
      local target="$HOME/$item"
      mkdir -p "$(dirname "$target")"
      if [[ ! -e "$target" ]]; then
        # Create with a tight umask so the file is 0600 from birth on
        # any user umask (Stage 5 round 1 finding 2 generalized).
        (umask 077 && printf '{}\n' >"$target") || cs::helpers::die 3 "could not seed home-linked file." "Failed to create $target." "  Check the directory permissions."
        chmod 600 "$target" || true
      fi
    done
  ) 9>"$lock_file"
}

__run_auto_trust_cwd() {
  local cwd=$1
  local lock_file=$2

  [[ "${CLAUDE_SESSION_AUTO_TRUST_CWD:-1}" == "1" ]] || return 0
  command -v jq >/dev/null 2>&1 || cs::helpers::die 3 "jq is required." "Auto-trust needs jq to edit Claude trust state." "  Install jq."
  command -v flock >/dev/null 2>&1 || cs::helpers::die 3 "flock is required." "Auto-trust needs flock to serialize trust-state updates." "  Install util-linux flock."

  local trust_file="$HOME/.claude.json"
  mkdir -p "$(dirname "$trust_file")"

  # Sentinel: the locked subshell touches this file if (and only if) it
  # flipped hasTrustDialogAccepted from absent/false to true on this
  # launch. Using a sentinel instead of a non-zero subshell exit keeps
  # the helper compatible with `set -e` in the wrapper entrypoint.
  local wrote_marker="$trust_file.cs-trust-wrote.$$"
  rm -f "$wrote_marker"
  (
    flock 9
    # __run_seed_home_link_files already seeded "$trust_file" with "{}"
    # under the same lock when absent; keep this guard as a safety net
    # for the unlikely case where .claude.json was removed between the
    # seed and this call.
    if [[ ! -e "$trust_file" ]]; then
      (umask 077 && printf '{}\n' >"$trust_file") || cs::helpers::die 3 "could not seed Claude trust state." "Failed to create $trust_file." "  Check the directory permissions."
      chmod 600 "$trust_file" || true
    fi
    # Probe prior trust state under the same lock so the notice only
    # fires when this launch actually flips the bit. Already-trusted
    # paths stay silent on subsequent launches.
    if jq -e --arg cwd "$cwd" '.projects[$cwd].hasTrustDialogAccepted == true' "$trust_file" >/dev/null 2>&1; then
      exit 0
    fi
    local tmp="$trust_file.tmp.$$"
    # Create the temp file with restrictive permissions before jq writes
    # to it; otherwise the user's umask leaks into the canonical
    # $HOME/.claude.json after `mv` and an existing 0600 file becomes
    # world-readable on a typical umask 022 (Stage 5 finding 2).
    (umask 077 && : >"$tmp") || cs::helpers::die 3 "could not stage Claude trust state." "Failed to create $tmp." "  Check the directory permissions."
    chmod 600 "$tmp" || true
    if jq --arg cwd "$cwd" '
      .projects = (.projects // {})
      | .projects[$cwd] = ((.projects[$cwd] // {}) + {
          hasTrustDialogAccepted: true,
          hasCompletedProjectOnboarding: true
        })
    ' "$trust_file" >"$tmp"; then
      mv -f "$tmp" "$trust_file"
      : >"$wrote_marker"
    else
      rm -f "$tmp"
      cs::helpers::die 3 "could not persist Claude trust state." "Failed to update $trust_file for $cwd." "  Check file permissions and JSON validity."
    fi
  ) 9>"$lock_file"
  if [[ -e "$wrote_marker" ]]; then
    rm -f "$wrote_marker"
    cs::helpers::log "auto-trusted current directory: $cwd (CLAUDE_SESSION_AUTO_TRUST_CWD=1; set to 0 to disable)"
  fi
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

  cs::helpers::source_fn resolve_profile
  cs::helpers::source_fn compose_profile
  cs::helpers::source_fn apply_profile_env
  cs::helpers::source_fn terminal_id
  cs::helpers::source_fn session_dir
  cs::helpers::source_fn sync_files
  cs::helpers::source_fn link_files
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

  if [[ "${CS_PROFILE_MODE:-stock}" == "manifest" ]]; then
    cs::fn::compose_profile "$CS_PROFILE_MANIFEST" "$session_dir"
    cs::fn::apply_profile_env "$session_dir/.claude-session-compose.json"
  else
    rm -f "$session_dir/settings.json" \
      "$session_dir/.claude-session-compose.json"
  fi

  local shared_dir=${CLAUDE_SESSION_SHARED_DIR:-$HOME/.claude}
  mkdir -p "$shared_dir"

  cs::fn::sync_files in "$session_dir" "$shared_dir"
  cs::fn::link_files "$session_dir" "$shared_dir"
  __run_write_meta "$session_dir/session-meta.json" "${CLAUDE_SESSION_PROFILE:-}" "$terminal_id" "$root" "$root_source" "$PWD"
  if [[ $dry_run -ne 1 ]]; then
    __run_seed_home_link_files "$shared_dir/.claude-session.lock"
    __run_auto_trust_cwd "$PWD" "$shared_dir/.claude-session.lock"
  fi

  local real_claude
  real_claude=$(cs::fn::real_claude)

  if [[ $bare -eq 0 && -z "${CLAUDE_CODE_OAUTH_TOKEN:-}" && -n "${CLAUDE_SESSION_OAUTH_CMD:-}" ]]; then
    local token=""
    if token=$(cs::fn::run_hook "$CLAUDE_SESSION_OAUTH_CMD" --timeout 5); then
      token=${token//$'\n'/}
      token=${token//$'\r'/}
      if [[ -n "${token//[[:space:]]/}" ]]; then
        CLAUDE_CODE_OAUTH_TOKEN=$token
        export CLAUDE_CODE_OAUTH_TOKEN
      fi
    fi
    # Hook failure or empty output: fall through and let the real claude
    # binary handle native auth (~/.claude/.credentials.json, keychain,
    # interactive /login, ANTHROPIC_API_KEY, apiKeyHelper).
  fi

  export CLAUDE_CONFIG_DIR=$session_dir
  export CLAUDE_SESSION_DIR=$session_dir
  export CLAUDE_SESSION_PROFILE=${CLAUDE_SESSION_PROFILE:-}

  if [[ $dry_run -eq 1 ]]; then
    local mode=${CS_PROFILE_MODE:-stock}
    local manifest=${CS_PROFILE_MANIFEST:-unset}
    local settings_path="$session_dir/settings.json"
    local layers=none
    local profile=${CLAUDE_SESSION_PROFILE:-stock}
    if [[ "$mode" == "stock" ]]; then
      manifest='unset'
      settings_path='not-written'
    elif [[ -f "$session_dir/.claude-session-compose.json" ]]; then
      layers=$(jq -r '.layers | join(":")' "$session_dir/.claude-session-compose.json" 2>/dev/null || printf 'none')
    fi
    printf 'mode=%s\n' "$mode"
    printf 'manifest=%s\n' "$manifest"
    printf 'profile=%s\n' "$profile"
    printf 'session_dir=%s\n' "$session_dir"
    printf 'settings=%s\n' "$settings_path"
    printf 'layers=%s\n' "$layers"
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
