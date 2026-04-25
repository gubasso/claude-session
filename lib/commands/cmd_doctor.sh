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

  local shared_dir=${CLAUDE_SESSION_SHARED_DIR:-$HOME/.claude}
  if [[ -d "$shared_dir" && -r "$shared_dir" ]]; then
    __doctor_line "shared dir" "OK" "$shared_dir (exists, readable)"
  else
    __doctor_line "shared dir" "FAIL" "$shared_dir missing or unreadable"
    fails=$((fails + 1))
    next="${next}  mkdir -p \"$shared_dir\""$'\n'
  fi

  cs::helpers::source_fn resolve_profile
  # Run the resolver in a subshell so its die/exit cannot abort doctor.
  local profile_err
  profile_err=$(mktemp)
  if (cs::fn::resolve_profile) 2>"$profile_err"; then
    __doctor_line "profile" "OK" "\"${CLAUDE_SESSION_PROFILE:-default}\""
    # Re-run in the current shell now that we know it succeeds, so the
    # rest of doctor sees the resolved profile env.
    cs::fn::resolve_profile
  else
    local prof=${CS_CLI_PROFILE:-${CLAUDE_SESSION_PROFILE:-default}}
    __doctor_line "profile" "FAIL" "profile \"$prof\" could not be resolved"
    fails=$((fails + 1))
    next="${next}  Run:  claude-session profile list"$'\n'
  fi
  rm -f "$profile_err"

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

  if [[ -n "${CLAUDE_SESSION_OAUTH_CMD:-}" ]]; then
    if command -v timeout >/dev/null 2>&1; then
      __doctor_line "oauth hook" "OK" "configured"
    else
      __doctor_line "oauth hook" "WARN" "configured, but timeout not found"
    fi
  else
    __doctor_line "oauth hook" "—" "CLAUDE_SESSION_OAUTH_CMD unset (skipped)"
  fi
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
  if command -v flock >/dev/null 2>&1; then
    __doctor_line "flock" "OK" "$(command -v flock)"
  else
    __doctor_line "flock" "FAIL" "flock not found"
    fails=$((fails + 1))
  fi
  if command -v timeout >/dev/null 2>&1; then
    __doctor_line "timeout" "OK" "$(command -v timeout)"
  else
    __doctor_line "timeout" "WARN" "timeout not found; OAuth hook timeout disabled"
  fi

  printf '\nSessions\n'
  cs::helpers::source_fn session_inventory
  cs::fn::session_inventory || true

  if [[ $verbose -eq 1 ]]; then
    printf '\nEnvironment\n'
    printf 'CLAUDE_SESSION_PROFILE=%s\n' "${CLAUDE_SESSION_PROFILE:-default}"
    printf 'CLAUDE_SESSION_CONFIG_DIR=%s\n' "$(cs::helpers::config_dir_default)"
    printf 'CLAUDE_SESSION_SHARED_DIR=%s\n' "$shared_dir"
    # Per docs/commands.md: also surface the env vars the active profile
    # would set (CLAUDE_CODE_*, ANTHROPIC_*, etc), with redaction unless
    # --verbose was *also* passed (which it is, here, so values are shown
    # except for *_TOKEN / *_SECRET / *_KEY / *_PASSWORD / OAUTH_CMD per
    # the secret-key rule).
    local config_dir
    config_dir=$(cs::helpers::config_dir_default)
    local profile=${CLAUDE_SESSION_PROFILE:-default}
    local profile_file="$config_dir/profiles/$profile.env"
    if [[ -f "$profile_file" ]]; then
      printf '# profile vars from %s\n' "$profile_file"
      local kv key value
      # shellcheck disable=SC2016  # script body for inner bash -c; vars expand in child
      while IFS= read -r kv; do
        [[ -n "$kv" ]] || continue
        [[ "$kv" == *=* ]] || continue
        key=${kv%%=*}
        value=${kv#*=}
        case "$key" in
          HOME | PATH | PWD | SHLVL | _ | OLDPWD | CS_PROFILE_FILE | CS_LIB_DIR) continue ;;
        esac
        # In verbose mode we still apply the suffix-based secret redaction
        # to keep tokens out of triage paste-bins.
        value=$(cs::helpers::redact "$key" "$value" 0)
        printf '%s=%s\n' "$key" "$value"
      done < <(env -i HOME="$HOME" PATH="$PATH" \
        CS_PROFILE_FILE="$profile_file" CS_LIB_DIR="${LIB_DIR:-}" \
        bash -c '
          # shellcheck source=/dev/null
          . "$CS_LIB_DIR/helpers.sh"
          # shellcheck source=/dev/null
          . "$CS_LIB_DIR/functions/fn_load_config.sh"
          cs::fn::__apply_dotenv "$CS_PROFILE_FILE"
          env -0 | tr "\0" "\n"
        ' 2>/dev/null) || true
    fi
  fi

  if [[ -n "$next" ]]; then
    printf '\nNext:\n%s' "$next"
  fi
  [[ $fails -eq 0 ]]
}
