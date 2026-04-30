# shellcheck shell=bash
: 'desc: Resolve the active claude-session profile manifest or stock mode.'

cs::fn::resolve_profile() {
  local config_dir
  config_dir=$(cs::helpers::config_dir_default)
  local explicit=0
  local profile=${CS_CLI_PROFILE:-${CLAUDE_SESSION_PROFILE:-}}
  local manifest=""
  if [[ -n "${CS_CLI_PROFILE:-}" || -n "${CLAUDE_SESSION_PROFILE:-}" ]]; then
    explicit=1
  fi

  if [[ -n "$profile" ]]; then
    manifest="$config_dir/profiles/$profile.yaml"
  elif [[ -f "$config_dir/profiles/default.yaml" ]]; then
    profile=default
    manifest="$config_dir/profiles/default.yaml"
  fi

  if [[ $explicit -eq 1 && ! -f "$manifest" ]]; then
    local found=""
    if [[ -d "$config_dir/profiles" ]]; then
      found=$(find "$config_dir/profiles" -maxdepth 1 -type f -name '*.yaml' -printf '%f\n' 2>/dev/null | sed 's/\.yaml$//' | sort | paste -sd ', ' -)
    fi
    cs::helpers::die 6 "profile \"$profile\" not found." "No manifest at $manifest. Found profiles: ${found:-none}." "  List available profiles:  claude-session profile list"$'\n'"  Create the profile:       \${EDITOR:-vi} \"$manifest\"" "claude-session profile list"
  fi

  CLAUDE_SESSION_PROFILE=$profile
  CLAUDE_SESSION_CONFIG_DIR=$config_dir
  if [[ -n "$manifest" ]]; then
    CS_PROFILE_MODE=manifest
    CS_PROFILE_MANIFEST=$manifest
  else
    CS_PROFILE_MODE=stock
    CS_PROFILE_MANIFEST=""
  fi
  export CLAUDE_SESSION_PROFILE CLAUDE_SESSION_CONFIG_DIR CS_PROFILE_MODE CS_PROFILE_MANIFEST
}
