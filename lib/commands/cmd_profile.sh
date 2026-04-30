# shellcheck shell=bash
: 'desc: List profiles or show one profile manifest and merged env.'

__profile_help() {
  cat <<'EOF'
USAGE:
  claude-session profile list
  claude-session profile show <name> [--verbose]
EOF
}

__profile_list() {
  local config_dir
  config_dir=$(cs::helpers::config_dir_default)
  local dir="$config_dir/profiles"
  [[ -e "$dir" ]] || return 0
  [[ -r "$dir" ]] || cs::helpers::die 3 "profiles directory unreadable." "Cannot read $dir." "  chmod u+r \"$dir\""
  local active=${CLAUDE_SESSION_PROFILE:-}
  local file=""
  while IFS= read -r file; do
    [[ -n "$file" ]] || continue
    local name
    name=$(basename "$file" .yaml)
    if [[ -n "$active" && "$name" == "$active" ]]; then
      printf '* %s\n' "$name"
    else
      printf '  %s\n' "$name"
    fi
  done < <(find "$dir" -maxdepth 1 -type f -name '*.yaml' -print | sort)
}

__profile_show() {
  local name=$1
  local verbose=$2
  local config_dir
  config_dir=$(cs::helpers::config_dir_default)
  local manifest="$config_dir/profiles/$name.yaml"
  [[ -f "$manifest" ]] || cs::helpers::die 6 "profile \"$name\" not found." "No manifest at $manifest." "  List available profiles:  claude-session profile list" "claude-session profile list"
  cs::helpers::source_fn compose_profile
  local tmpdir
  tmpdir=$(mktemp -d "${TMPDIR:-/tmp}/claude-session-profile.XXXXXX")
  cs::fn::compose_profile "$manifest" "$tmpdir"
  local sidecar="$tmpdir/.claude-session-compose.json"
  printf '# profile: %s\n' "$name"
  printf '# manifest: %s\n' "$manifest"
  printf '# layers:\n'
  local idx=1
  local layer=""
  while IFS= read -r layer; do
    [[ -n "$layer" ]] || continue
    printf '#   %s: %s\n' "$idx" "$layer"
    idx=$((idx + 1))
  done < <(jq -r '.layers[]' "$sidecar")
  command -v base64 >/dev/null 2>&1 || cs::helpers::die 3 "base64 is required." \
    "claude-session uses base64 to decode merged-env values from the compose sidecar." \
    "  Install GNU coreutils (or your platform's base64) and re-run claude-session doctor."
  local key b64 value
  while IFS=$'\t' read -r key b64; do
    [[ -n "$key" ]] || continue
    # NUL-terminated read preserves trailing newlines through the base64 decode.
    IFS= read -r -d '' value < <(printf '%s' "$b64" | base64 -d && printf '\0') \
      || cs::helpers::die 3 "compose sidecar contains an undecodable env value." \
        "Failed to base64-decode the value for \"$key\" in $sidecar." \
        "  Re-run claude-session doctor and inspect the merged settings layers."
    value=$(cs::helpers::redact "$key" "$value" "$verbose")
    printf '%s=%s\n' "$key" "$value"
  done < <(jq -r '.env | to_entries[]? | "\(.key)\t\((.value | tostring) | @base64)"' "$sidecar")
  rm -rf "$tmpdir"
}

cs::cmd::profile() {
  local sub=${1:-}
  [[ $# -gt 0 ]] && shift
  case "$sub" in
    list)
      [[ $# -eq 0 ]] || cs::helpers::die 2 "profile list takes no arguments." "Unexpected argument: $1" "  claude-session profile list"
      __profile_list
      ;;
    show)
      local name=${1:-}
      [[ -n "$name" ]] || cs::helpers::die 2 "missing profile name." "profile show requires a profile name." "  claude-session profile show default"
      shift
      local verbose=0
      while [[ $# -gt 0 ]]; do
        case "$1" in
          --verbose) verbose=1 ;;
          --help | -h)
            __profile_help
            return 0
            ;;
          *) cs::helpers::die 2 "unknown profile flag \"$1\"." "The profile command did not understand \"$1\"." "  claude-session profile --help" ;;
        esac
        shift
      done
      __profile_show "$name" "$verbose"
      ;;
    --help | -h | "") __profile_help ;;
    *) cs::helpers::die 2 "unknown profile subcommand \"$sub\"." "Expected list or show." "  claude-session profile --help" ;;
  esac
}
