# shellcheck shell=bash
: 'desc: Resolve and load the active claude-session profile.'

cs::fn::resolve_profile() {
  local config_dir
  config_dir=$(cs::helpers::config_dir_default)
  local shared_dir=${CLAUDE_SESSION_SHARED_DIR:-$HOME/.claude}
  local profile=${CLAUDE_SESSION_PROFILE:-default}
  local explicit=0
  if [[ -n "${CS_CLI_PROFILE:-}" || -n "${CLAUDE_SESSION_PROFILE:-}" ]]; then
    explicit=1
  fi
  if [[ -n "${CS_CLI_PROFILE:-}" ]]; then
    profile=$CS_CLI_PROFILE
  fi

  local profile_file="$config_dir/profiles/$profile.env"
  local config_overlay="$config_dir/profiles/$profile.settings.json"
  local shared_overlay="$shared_dir/settings.$profile.json"

  if [[ -f "$profile_file" ]]; then
    cs::helpers::source_fn load_config
    cs::fn::validate_dotenv "$profile_file"
    cs::fn::__apply_dotenv "$profile_file"
  elif [[ $explicit -eq 1 && ! -f "$config_overlay" && ! -f "$shared_overlay" ]]; then
    local found=""
    if [[ -d "$config_dir/profiles" ]]; then
      found=$(find "$config_dir/profiles" -maxdepth 1 -type f -name '*.env' -printf '%f\n' 2>/dev/null | sed 's/\.env$//' | sort | paste -sd ', ' -)
    fi
    cs::helpers::die 6 "profile \"$profile\" not found." "No file at $profile_file and no overlay at $config_overlay or $shared_overlay. Found profiles: ${found:-none}." "  List available profiles:  claude-session profile list"$'\n'"  Create the profile:       \${EDITOR:-vi} \"$profile_file\"" "claude-session profile list"
  fi

  CLAUDE_SESSION_PROFILE=$profile
  export CLAUDE_SESSION_PROFILE
  export CLAUDE_SESSION_CONFIG_DIR=$config_dir
}
