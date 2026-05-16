# shellcheck shell=bash
: 'desc: Run structured health checks for claude-session.'

__doctor_help() {
  cat <<'EOF'
USAGE:
  claude-session doctor [--verbose]
EOF
}

__doctor_line() {
  printf '%-14s %-5s %s\n' "$1" "$2" "$3"
}

cs::cmd::doctor() {
  local verbose=${CLAUDE_SESSION_VERBOSE:-0}
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --verbose) verbose=1 ;;
      --help | -h)
        __doctor_help
        return 0
        ;;
      *) cs::helpers::die 2 "unknown doctor flag \"$1\"." "The doctor command did not understand \"$1\"." "  claude-session doctor --help" ;;
    esac
    shift
  done

  local fails=0
  local next=""
  local config_file
  config_file=$(cs::helpers::config_path)
  local config_dir
  config_dir=$(cs::helpers::config_dir_default)
  local manifests_exist=0
  if [[ -d "$config_dir/profiles" ]] && find "$config_dir/profiles" -maxdepth 1 -type f -name '*.yaml' -print -quit | grep -q .; then
    manifests_exist=1
  fi
  if [[ -f "$config_file" ]]; then
    local mode
    mode=$(stat -c '%a' "$config_file" 2>/dev/null || printf '000')
    if ((8#$mode > 600)); then
      __doctor_line "config" "WARN" "$config_file (mode $mode; consider chmod 600)"
      next="${next}  chmod 600 \"$config_file\""$'\n'
    else
      __doctor_line "config" "OK" "$config_file (mode $mode)"
    fi
  else
    __doctor_line "config" "WARN" "no config file found at $config_file"
  fi

  cs::helpers::source_fn resolve_profile
  cs::helpers::source_fn compose_profile
  cs::helpers::source_fn apply_profile_env
  # Run the resolver in a subshell so its die/exit cannot abort doctor.
  local profile_err
  profile_err=$(mktemp)
  if (cs::fn::resolve_profile) 2>"$profile_err"; then
    cs::fn::resolve_profile
    if [[ "${CS_PROFILE_MODE:-stock}" == "manifest" ]]; then
      local compose_dir
      compose_dir=$(mktemp -d "${TMPDIR:-/tmp}/claude-session-doctor.XXXXXX")
      local compose_err
      compose_err=$(mktemp)
      if (cs::fn::compose_profile "$CS_PROFILE_MANIFEST" "$compose_dir") 2>"$compose_err"; then
        cs::fn::compose_profile "$CS_PROFILE_MANIFEST" "$compose_dir"
        cs::fn::apply_profile_env "$compose_dir/.claude-session-compose.json"
        __doctor_line "profile" "OK" "\"${CLAUDE_SESSION_PROFILE:-}\""
        __doctor_line "mode" "OK" "manifest ($CS_PROFILE_MANIFEST)"
      else
        __doctor_line "profile" "FAIL" "$(tr '\n' ' ' <"$compose_err")"
        fails=$((fails + 1))
        next="${next}  Fix the manifest and layer files, then run:  claude-session doctor"$'\n'
      fi
      rm -f "$compose_err"
      export __CS_DOCTOR_COMPOSE_DIR="$compose_dir"
    else
      __doctor_line "profile" "OK" "stock"
      __doctor_line "mode" "OK" "stock (no manifest)"
    fi
  else
    local prof=${CS_CLI_PROFILE:-${CLAUDE_SESSION_PROFILE:-}}
    __doctor_line "profile" "FAIL" "profile \"$prof\" could not be resolved"
    fails=$((fails + 1))
    next="${next}  Run:  claude-session profile list"$'\n'
  fi
  rm -f "$profile_err"

  if command -v yq >/dev/null 2>&1; then
    __doctor_line "yq" "OK" "$(command -v yq)"
  elif [[ $manifests_exist -eq 1 ]]; then
    __doctor_line "yq" "FAIL" "yq not found"
    fails=$((fails + 1))
    next="${next}  Install yq from https://github.com/mikefarah/yq"$'\n'
  else
    __doctor_line "yq" "WARN" "yq not found; no manifests discovered"
  fi

  local shared_dir=${CLAUDE_SESSION_SHARED_DIR:-$HOME/.claude}
  if [[ -d "$shared_dir" && -r "$shared_dir" ]]; then
    __doctor_line "shared dir" "OK" "$shared_dir (exists, readable)"
  else
    __doctor_line "shared dir" "FAIL" "$shared_dir missing or unreadable"
    fails=$((fails + 1))
    next="${next}  mkdir -p \"$shared_dir\""$'\n'
  fi

  local home_trust="$HOME/.claude.json"
  if [[ -e "$home_trust" ]]; then
    local mode
    mode=$(stat -c '%a' "$home_trust" 2>/dev/null || printf '000')
    local mount_note="regular file (rename fast path)"
    if command -v findmnt >/dev/null 2>&1 \
      && findmnt -T "$home_trust" --target "$home_trust" >/dev/null 2>&1 \
      && [[ "$(findmnt -no TARGET -T "$home_trust" 2>/dev/null)" == "$home_trust" ]]; then
      mount_note="bind-mounted leaf (in-place rewrite path active)"
    fi
    __doctor_line "home trust" "OK" "$home_trust (mode $mode; $mount_note)"
  else
    __doctor_line "home trust" "WARN" "$home_trust missing (will be seeded on next run)"
  fi

  cs::helpers::source_fn real_claude
  local real=""
  local err
  err=$(mktemp)
  if real=$(cs::fn::real_claude 2>"$err"); then
    __doctor_line "real claude" "OK" "$real"
  else
    __doctor_line "real claude" "FAIL" "$(tr '\n' ' ' <"$err")"
    fails=$((fails + 1))
    next="${next}  Set CLAUDE_SESSION_REAL_CLAUDE=/path/to/claude"$'\n'
  fi
  rm -f "$err"

  if [[ -n "${CLAUDE_SESSION_POST_EXIT_CMD:-}" ]]; then
    __doctor_line "post-exit hook" "OK" "configured"
  else
    __doctor_line "post-exit hook" "—" "CLAUDE_SESSION_POST_EXIT_CMD unset (skipped)"
  fi

  cs::helpers::source_fn session_dir
  local root=""
  err=$(mktemp)
  if root=$(cs::fn::session_dir 2>"$err"); then
    local root_source
    root_source=$(cs::fn::session_root_source_for "$root")
    if [[ "$root_source" == "runtime" ]]; then
      __doctor_line "session root" "OK" "$root/  (XDG_RUNTIME_DIR, mode 700)"
    else
      __doctor_line "session root" "WARN" "$root/  (XDG_STATE_HOME fallback)"
      next="${next}  Restore or set XDG_RUNTIME_DIR"$'\n'
    fi
  else
    __doctor_line "session root" "FAIL" "$(tr '\n' ' ' <"$err")"
    fails=$((fails + 1))
  fi
  rm -f "$err"

  if command -v jq >/dev/null 2>&1; then
    __doctor_line "jq" "OK" "$(command -v jq)"
  else
    __doctor_line "jq" "FAIL" "jq not found"
    fails=$((fails + 1))
  fi
  if command -v base64 >/dev/null 2>&1; then
    __doctor_line "base64" "OK" "$(command -v base64)"
  else
    __doctor_line "base64" "FAIL" "base64 not found (needed to decode merged env values)"
    fails=$((fails + 1))
  fi
  if command -v flock >/dev/null 2>&1; then
    __doctor_line "flock" "OK" "$(command -v flock)"
  else
    __doctor_line "flock" "FAIL" "flock not found"
    fails=$((fails + 1))
  fi
  local lock_file="$shared_dir/.claude-session.lock"
  if [[ -e "$lock_file" ]]; then
    local lock_inode
    lock_inode=$(stat -c '%i' "$lock_file" 2>/dev/null || printf 'unknown')
    __doctor_line "shared lock" "OK" "$lock_file (inode $lock_inode)"
  else
    __doctor_line "shared lock" "—" "$lock_file (will be created on next run)"
  fi
  printf '\nSessions\n'
  cs::helpers::source_fn session_inventory
  cs::fn::session_inventory || true

  if [[ $verbose -eq 1 ]]; then
    printf '\nEnvironment\n'
    printf 'CLAUDE_SESSION_PROFILE=%s\n' "${CLAUDE_SESSION_PROFILE:-}"
    printf 'CLAUDE_SESSION_CONFIG_DIR=%s\n' "$config_dir"
    printf 'CLAUDE_SESSION_CACHE_DIR=%s\n' "$(cs::helpers::cache_dir_default)"
    printf 'CLAUDE_SESSION_SHARED_DIR=%s\n' "$shared_dir"
    printf 'HOME_TRUST_FILE=%s\n' "$HOME/.claude.json"
    if [[ -n "${__CS_DOCTOR_COMPOSE_DIR:-}" && -f "${__CS_DOCTOR_COMPOSE_DIR}/.claude-session-compose.json" ]]; then
      printf '# merged env from %s\n' "${__CS_DOCTOR_COMPOSE_DIR}/.claude-session-compose.json"
      if ! command -v base64 >/dev/null 2>&1; then
        # The dependency check above already FAILed the run; surface here too
        # so the verbose dump's omission is explicit, not silent.
        printf '# merged env not shown: base64 is required to decode the compose sidecar.\n'
      else
        local key b64 value
        while IFS=$'\t' read -r key b64; do
          [[ -n "$key" ]] || continue
          # NUL-terminated read preserves trailing newlines through the base64 decode.
          IFS= read -r -d '' value < <(printf '%s' "$b64" | base64 -d && printf '\0') \
            || cs::helpers::die 3 "compose sidecar contains an undecodable env value." \
              "Failed to base64-decode the value for \"$key\" in ${__CS_DOCTOR_COMPOSE_DIR}/.claude-session-compose.json." \
              "  Re-run claude-session doctor and inspect the merged settings layers."
          value=$(cs::helpers::redact "$key" "$value" 0)
          printf '%s=%s\n' "$key" "$value"
        done < <(jq -r '.env | to_entries[]? | "\(.key)\t\((.value | tostring) | @base64)"' "${__CS_DOCTOR_COMPOSE_DIR}/.claude-session-compose.json")
      fi
    fi
  fi

  if [[ -n "$next" ]]; then
    printf '\nNext:\n%s' "$next"
  fi
  if [[ -n "${__CS_DOCTOR_COMPOSE_DIR:-}" ]]; then
    rm -rf "$__CS_DOCTOR_COMPOSE_DIR"
    unset __CS_DOCTOR_COMPOSE_DIR
  fi
  [[ $fails -eq 0 ]]
}
