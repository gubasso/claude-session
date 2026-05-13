# shellcheck shell=bash
: 'desc: Compose layered profile settings and emit a compose sidecar.'

__require_yq() {
  command -v yq >/dev/null 2>&1 || cs::helpers::die 3 "yq is required." "Profile manifests exist, so claude-session must parse YAML with yq." "  Install yq from https://github.com/mikefarah/yq"
}

__require_jq() {
  command -v jq >/dev/null 2>&1 || cs::helpers::die 3 "jq is required." "Profile settings layers exist, so claude-session must merge JSON with jq." "  Install jq."
}

__file_mtime() {
  local file=$1
  stat -c '%Y' "$file" 2>/dev/null || printf '0'
}

__file_size() {
  local file=$1
  stat -c '%s' "$file" 2>/dev/null || printf '0'
}

__cache_hash() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum | awk '{print $1}'
  else
    cksum | awk '{print $1}'
  fi
}

__validate_manifest() {
  local manifest=$1
  [[ -f "$manifest" ]] || cs::helpers::die 3 "profile manifest not found." "Cannot read $manifest." "  Create the manifest and re-run claude-session profile list."
  yq eval '.' "$manifest" >/dev/null 2>&1 || cs::helpers::die 3 "profile manifest is invalid." "yq could not parse $manifest." "  Fix the YAML manifest and re-run claude-session doctor."
  local root_type
  root_type=$(yq eval 'type' "$manifest" 2>/dev/null || true)
  [[ "$root_type" == "!!map" ]] || cs::helpers::die 3 "profile manifest is invalid." "$manifest must contain a YAML mapping at the top level." "  Fix the YAML manifest and re-run claude-session doctor."
  local layers_type
  layers_type=$(yq eval '.settings-layers | type' "$manifest" 2>/dev/null || true)
  [[ "$layers_type" == "!!seq" ]] || cs::helpers::die 3 "profile manifest is invalid." "$manifest: settings-layers must be an array." "  Fix the YAML manifest and re-run claude-session doctor."
  local layers_len
  layers_len=$(yq eval '.settings-layers | length' "$manifest" 2>/dev/null || true)
  [[ "${layers_len:-0}" -gt 0 ]] || cs::helpers::die 3 "profile manifest is invalid." "$manifest: settings-layers must not be empty." "  Add at least one settings layer and re-run claude-session doctor."
  local extra_keys=""
  extra_keys=$(yq eval 'keys | .[]' "$manifest" 2>/dev/null | grep -vx 'settings-layers' || true)
  [[ -z "$extra_keys" ]] || cs::helpers::die 3 "profile manifest is invalid." "$manifest contains unsupported top-level keys: $(printf '%s' "$extra_keys" | paste -sd ', ' -)." "  Remove the unsupported keys and re-run claude-session doctor."
}

__manifest_layer_names() {
  local manifest=$1
  # Reject non-string entries (e.g. integers, booleans) per the manifest schema:
  # settings-layers must be an ordered list of strings. yq's text/tsv output coerces
  # numbers and booleans to their textual form, so we must check the YAML tag per item.
  local non_str
  non_str=$(yq eval '[.settings-layers[] | tag] | .[]' "$manifest" 2>/dev/null \
    | grep -vx '!!str' || true)
  if [[ -n "$non_str" ]]; then
    cs::helpers::die 3 "profile manifest is invalid." \
      "$manifest: settings-layers entries must be strings." \
      "  Quote layer names in the YAML manifest and re-run claude-session doctor."
  fi
  yq eval -o=tsv '.settings-layers[]' "$manifest" 2>/dev/null
}

__layer_paths() {
  local config_dir=$1
  shift
  local layer=""
  for layer in "$@"; do
    [[ -n "$layer" ]] || cs::helpers::die 3 "profile manifest is invalid." "settings-layers entries must not be empty." "  Fix the YAML manifest and re-run claude-session doctor."
    [[ "$layer" =~ ^[A-Za-z0-9._-]+$ ]] || cs::helpers::die 3 "profile manifest is invalid." "settings layer \"$layer\" has an invalid name." "  Use only letters, digits, dots, underscores, and hyphens."
    local path="$config_dir/settings/$layer.json"
    [[ -f "$path" ]] || cs::helpers::die 3 "settings layer \"$layer\" not found." "Expected layer file at $path." "  Create the layer file and re-run claude-session doctor."
    printf '%s\n' "$path"
  done
}

__cache_key() {
  local manifest=$1
  shift
  local manifest_mtime
  manifest_mtime=$(__file_mtime "$manifest")
  printf '%s|%s|%s' "$manifest" "$manifest_mtime" "$(__file_size "$manifest")"
  local path=""
  for path in "$@"; do
    # Include size alongside mtime so same-second content swaps (the
    # runtime-cache writeback can preserve a source mtime via `cp -p`,
    # and 1s mtime granularity hides same-second edits) still invalidate
    # the per-session compose cache.
    printf '|%s|%s|%s' "$path" "$(__file_mtime "$path")" "$(__file_size "$path")"
  done
  printf '\n'
}

__extract_env_json() {
  local settings_file=$1
  local env_json
  env_json=$(jq -c '.env // {}' "$settings_file") || cs::helpers::die 3 "settings JSON is invalid." "jq could not read the merged settings output." "  Fix the settings layers and re-run claude-session doctor."
  jq -e '. | type == "object"' >/dev/null <<<"$env_json" || cs::helpers::die 3 "settings.env must be an object." "Merged settings produced a non-object env block." "  Fix the settings layers and re-run claude-session doctor."
  printf '%s\n' "$env_json"
}

cs::fn::compose_profile() {
  local manifest=$1
  local session_dir=$2
  __require_yq
  __require_jq
  __validate_manifest "$manifest"

  local config_dir=${CLAUDE_SESSION_CONFIG_DIR:-$(cs::helpers::config_dir_default)}
  local -a layer_names=()
  local -a layer_paths=()
  local -a all_layer_paths=()
  local layer_names_output=""
  local layer_paths_output=""
  layer_names_output=$(__manifest_layer_names "$manifest")
  mapfile -t layer_names <<<"$layer_names_output"
  layer_paths_output=$(__layer_paths "$config_dir" "${layer_names[@]}")
  mapfile -t layer_paths <<<"$layer_paths_output"

  local cache_dir
  cache_dir=$(cs::helpers::cache_dir_default)
  local cache_settings="$cache_dir/settings.json"
  if [[ -f "$cache_settings" ]]; then
    if jq -e 'type == "object"' "$cache_settings" >/dev/null 2>&1; then
      all_layer_paths+=("$cache_settings")
    else
      cs::helpers::log "warning: ignoring corrupt $cache_settings (not a JSON object)"
    fi
  fi
  all_layer_paths+=("${layer_paths[@]}")

  local target_dir=$session_dir
  if [[ "$target_dir" == "-" ]]; then
    target_dir=$(mktemp -d "${TMPDIR:-/tmp}/claude-session-compose.XXXXXX")
  else
    mkdir -p "$target_dir"
  fi

  local settings_file="$target_dir/settings.json"
  local sidecar_file="$target_dir/.claude-session-compose.json"
  local cache_file="$target_dir/.claude-session-settings-cache"

  # Combine the path|mtime|size key with the runtime cache layer's content,
  # because sync_files uses `cp -p` to persist Claude's mutated settings.json
  # and 1s mtime granularity hides same-second, same-size content swaps.
  local _compose_key_content=""
  _compose_key_content=$(__cache_key "$manifest" "${all_layer_paths[@]}")
  if [[ -f "$cache_settings" ]]; then
    _compose_key_content+=$'\n'
    _compose_key_content+=$(cat "$cache_settings")
  fi

  if [[ "$session_dir" != "-" ]]; then
    local cache_key
    cache_key=$(printf '%s' "$_compose_key_content" | __cache_hash)
    if [[ -f "$settings_file" && -f "$sidecar_file" && -f "$cache_file" && "$(cat "$cache_file")" == "$cache_key" ]]; then
      cs::helpers::debug "settings cache hit for manifest $manifest"
      return 0
    fi
  fi

  local filter='.[0]'
  local idx=1
  while [[ $idx -lt ${#all_layer_paths[@]} ]]; do
    filter="$filter * .[$idx]"
    idx=$((idx + 1))
  done

  local tmp_settings="$target_dir/settings.json.tmp.$$"
  jq -s "$filter" "${all_layer_paths[@]}" >"$tmp_settings" || cs::helpers::die 3 "settings merge failed." "jq could not merge the settings layers declared by $manifest." "  Fix invalid JSON in the settings layers and re-run claude-session doctor."
  jq empty "$tmp_settings" >/dev/null || cs::helpers::die 3 "settings JSON is invalid." "Merged settings output failed jq validation." "  Fix the settings layers and re-run claude-session doctor."
  jq -e 'type == "object"' "$tmp_settings" >/dev/null || cs::helpers::die 3 "settings JSON is invalid." "Merged settings root must be a JSON object." "  Each settings layer must contain a JSON object at the top level."

  local env_json
  env_json=$(__extract_env_json "$tmp_settings")

  mv -f "$tmp_settings" "$settings_file"
  jq -n \
    --arg manifest "$manifest" \
    --argjson layers "$(printf '%s\n' "${all_layer_paths[@]}" | jq -R . | jq -s .)" \
    --argjson env "$env_json" \
    '{manifest: $manifest, layers: $layers, env: $env}' >"$sidecar_file" || cs::helpers::die 3 "compose sidecar write failed." "claude-session could not write $sidecar_file." "  Re-run claude-session doctor."

  if [[ "$session_dir" != "-" ]]; then
    # Recompute the runtime-cache-content-augmented key after the merge so the
    # short-circuit check uses the same shape on the next compose call.
    local final_key_content
    final_key_content=$(__cache_key "$manifest" "${all_layer_paths[@]}")
    if [[ -f "$cache_settings" ]]; then
      final_key_content+=$'\n'
      final_key_content+=$(cat "$cache_settings")
    fi
    printf '%s' "$final_key_content" | __cache_hash >"$cache_file"
  fi
}
