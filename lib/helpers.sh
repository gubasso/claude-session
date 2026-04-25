# shellcheck shell=bash
: 'desc: Shared low-level utilities for logging, errors, loading, and redaction.'

cs::helpers::source_fn() {
  local name=$1
  local path="$LIB_DIR/functions/fn_${name}.sh"
  if ! declare -F "cs::fn::${name}" >/dev/null 2>&1; then
    # shellcheck source=/dev/null
    . "$path"
  fi
}

cs::helpers::log() {
  printf '[claude-session] %s\n' "$*" >&2
}

cs::helpers::debug() {
  if [[ "${CLAUDE_SESSION_VERBOSE:-0}" == "1" ]]; then
    cs::helpers::log "debug: $*"
  fi
}

cs::helpers::die() {
  local code=$1
  local summary=$2
  local detail=$3
  local fix=$4
  local next=${5:-}
  cs::helpers::source_fn error
  cs::fn::error "$summary" "$detail" "$fix" "$next"
  exit "$code"
}

cs::helpers::require() {
  local cmd=$1
  command -v "$cmd" >/dev/null 2>&1
}

cs::helpers::is_secret_key() {
  local key=$1
  # Suffix-based secret detection per docs/config.md §"Secrets discipline".
  # docs/commands.md additionally shows CLAUDE_SESSION_OAUTH_CMD redacted by
  # default in `config show` example output, since OAuth-lookup commands
  # commonly embed secret-store paths.
  case "$key" in
    *_TOKEN | *_SECRET | *_KEY | *_PASSWORD) return 0 ;;
    CLAUDE_SESSION_OAUTH_CMD) return 0 ;;
  esac
  return 1
}

cs::helpers::redact() {
  local key=$1
  local value=$2
  local verbose=${3:-0}
  # Never redact a placeholder for an unconfigured variable — `(unset)`
  # and `(unset, auto-discover)` are documented `config show` outputs
  # (see docs/commands.md), and hiding them would obscure the fact that
  # nothing is configured.
  case "$value" in
    "(unset)" | "(unset, auto-discover)" | "")
      printf '%s\n' "$value"
      return 0
      ;;
  esac
  if [[ "$verbose" != "1" ]] && cs::helpers::is_secret_key "$key"; then
    printf '<redacted>\n'
  else
    printf '%s\n' "$value"
  fi
}

cs::helpers::config_dir_default() {
  if [[ -n "${CLAUDE_SESSION_CONFIG_DIR:-}" ]]; then
    printf '%s\n' "$CLAUDE_SESSION_CONFIG_DIR"
  else
    if [[ -z "${HOME:-}" ]]; then
      cs::helpers::die 3 "HOME is unset." "Cannot resolve the default XDG config directory." "  Set CLAUDE_SESSION_CONFIG_DIR=/path/to/config"
    fi
    printf '%s/claude-session\n' "${XDG_CONFIG_HOME:-$HOME/.config}"
  fi
}

cs::helpers::config_path() {
  if [[ -n "${CS_CLI_CONFIG:-}" ]]; then
    printf '%s\n' "$CS_CLI_CONFIG"
  else
    printf '%s/config.env\n' "$(cs::helpers::config_dir_default)"
  fi
}

cs::helpers::json_escape() {
  local s=$1
  s=${s//\\/\\\\}
  s=${s//\"/\\\"}
  s=${s//$'\n'/\\n}
  s=${s//$'\r'/\\r}
  s=${s//$'\t'/\\t}
  printf '%s\n' "$s"
}

cs::helpers::split_colon() {
  local value=$1
  local -n out_ref="$2"
  local old_ifs=$IFS
  IFS=:
  # shellcheck disable=SC2034  # out_ref is a nameref; assignment populates caller's array
  read -r -a out_ref <<<"$value"
  IFS=$old_ifs
}
