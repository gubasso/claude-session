# shellcheck shell=bash
: 'desc: Inspect, locate, or edit claude-session configuration.'

__config_help() {
  cat <<'EOF'
USAGE:
  claude-session config show [--verbose]
  claude-session config path
  claude-session config edit
EOF
}

__config_show() {
  local verbose=${1:-0}
  local keys=(
    CLAUDE_SESSION_CONFIG_DIR
    CLAUDE_SESSION_SHARED_DIR
    CLAUDE_SESSION_PROFILE
    CLAUDE_SESSION_REAL_CLAUDE
    CLAUDE_SESSION_OAUTH_CMD
    CLAUDE_SESSION_POST_EXIT_CMD
    CLAUDE_SESSION_SYNC_FILES
    CLAUDE_SESSION_LINK_FILES
    CLAUDE_SESSION_LINK_DIRS
    CLAUDE_SESSION_VERBOSE
  )
  local key=""
  local value=""
  for key in "${keys[@]}"; do
    case "$key" in
      CLAUDE_SESSION_CONFIG_DIR) value=${CLAUDE_SESSION_CONFIG_DIR:-$(cs::helpers::config_dir_default)} ;;
      CLAUDE_SESSION_SHARED_DIR) value=${CLAUDE_SESSION_SHARED_DIR:-$HOME/.claude} ;;
      CLAUDE_SESSION_PROFILE) value=${CLAUDE_SESSION_PROFILE:-default} ;;
      CLAUDE_SESSION_REAL_CLAUDE) value=${CLAUDE_SESSION_REAL_CLAUDE:-"(unset, auto-discover)"} ;;
      CLAUDE_SESSION_SYNC_FILES) value=${CLAUDE_SESSION_SYNC_FILES:-.credentials.json} ;;
      CLAUDE_SESSION_LINK_FILES) value=${CLAUDE_SESSION_LINK_FILES:-settings.local.json:keybindings.json:CLAUDE.md} ;;
      CLAUDE_SESSION_LINK_DIRS) value=${CLAUDE_SESSION_LINK_DIRS:-skills:agents:rules:commands:hooks} ;;
      CLAUDE_SESSION_VERBOSE) value=${CLAUDE_SESSION_VERBOSE:-0} ;;
      *) value=${!key:-"(unset)"} ;;
    esac
    value=$(cs::helpers::redact "$key" "$value" "$verbose")
    printf '%s=%s\n' "$key" "$value"
  done
}

__config_path() {
  cs::helpers::config_path
}

__config_edit() {
  local file
  file=$(cs::helpers::config_path)
  local dir
  dir=$(dirname "$file")
  mkdir -p "$dir" || cs::helpers::die 3 "could not create config directory." "Failed to create $dir." "  Check parent directory permissions."
  chmod 700 "$dir" || true
  if [[ ! -e "$file" ]]; then
    : >"$file" || cs::helpers::die 3 "could not create config file." "Failed to create $file." "  Check directory permissions."
    chmod 600 "$file" || true
  fi
  "${EDITOR:-${VISUAL:-vi}}" "$file"
}

cs::cmd::config() {
  local sub=${1:-}
  [[ $# -gt 0 ]] && shift
  local verbose=0
  local arg=""
  for arg in "$@"; do
    case "$arg" in
      --verbose) verbose=1 ;;
      --help | -h)
        __config_help
        return 0
        ;;
      *) cs::helpers::die 2 "unknown config flag \"$arg\"." "The config command did not understand \"$arg\"." "  claude-session config --help" ;;
    esac
  done
  case "$sub" in
    show) __config_show "$verbose" ;;
    path) __config_path ;;
    edit) __config_edit ;;
    --help | -h | "") __config_help ;;
    *) cs::helpers::die 2 "unknown config subcommand \"$sub\"." "Expected show, path, or edit." "  claude-session config --help" ;;
  esac
}
