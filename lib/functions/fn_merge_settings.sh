# shellcheck shell=bash
: 'desc: Merge base and profile Claude settings into the session directory.'

__file_mtime() {
  local file=$1
  stat -c '%Y' "$file" 2>/dev/null || printf '0'
}

__cache_hash() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum | awk '{print $1}'
  else
    cksum | awk '{print $1}'
  fi
}

cs::fn::merge_settings() {
  local shared_dir=$1
  local profile=$2
  local session_dir=$3
  local config_dir=$4
  local base="$shared_dir/settings.base.json"
  [[ -f "$base" ]] || return 0

  command -v jq >/dev/null 2>&1 || cs::helpers::die 3 "jq is required." "settings.base.json exists, so claude-session must merge settings with jq." "  Install jq or remove $base."

  local overlay=""
  if [[ -f "$config_dir/profiles/$profile.settings.json" ]]; then
    overlay="$config_dir/profiles/$profile.settings.json"
  elif [[ -f "$shared_dir/settings.$profile.json" ]]; then
    overlay="$shared_dir/settings.$profile.json"
  fi

  local base_m
  local overlay_m=0
  base_m=$(__file_mtime "$base")
  if [[ -n "$overlay" ]]; then
    overlay_m=$(__file_mtime "$overlay")
  fi
  local cache_key
  cache_key=$(printf '%s|%s|%s|%s|%s\n' "$profile" "$base" "$base_m" "$overlay" "$overlay_m" | __cache_hash)
  local cache_file="$session_dir/.claude-session-settings-cache"
  if [[ -f "$session_dir/settings.json" && -f "$cache_file" && "$(cat "$cache_file")" == "$cache_key" ]]; then
    cs::helpers::debug "settings cache hit for profile $profile"
    return 0
  fi

  local tmp="$session_dir/settings.json.tmp.$$"
  if [[ -n "$overlay" ]]; then
    jq -s '.[0] * .[1]' "$base" "$overlay" >"$tmp" || cs::helpers::die 3 "settings merge failed." "jq could not merge $base with $overlay." "  Fix invalid JSON in the settings files." "claude-session doctor"
  else
    cp "$base" "$tmp"
  fi
  jq empty "$tmp" >/dev/null || cs::helpers::die 3 "settings JSON is invalid." "Merged settings output failed jq validation." "  Fix $base or the profile overlay." "claude-session doctor"
  mv -f "$tmp" "$session_dir/settings.json"
  printf '%s\n' "$cache_key" >"$cache_file"
}
