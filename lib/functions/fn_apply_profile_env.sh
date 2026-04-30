# shellcheck shell=bash
: 'desc: Apply merged profile environment variables from a compose sidecar.'

cs::fn::apply_profile_env() {
  local sidecar=$1
  [[ -f "$sidecar" ]] || cs::helpers::die 3 "compose sidecar not found." "Cannot read $sidecar." "  Re-run claude-session doctor."
  # base64 is a hard runtime dependency (used by the lossless transport that
  # preserves tabs/newlines/trailing-newlines in merged-env values). Fail fast
  # when it is absent so we never silently apply a partial environment.
  command -v base64 >/dev/null 2>&1 || cs::helpers::die 3 "base64 is required." \
    "claude-session uses base64 to decode merged-env values from the compose sidecar." \
    "  Install GNU coreutils (or your platform's base64) and re-run claude-session doctor."
  # Stream entries as TAB-delimited <key>\t<base64-of-value>. Base64 keeps tabs and
  # newlines (including trailing newlines) inside values intact end-to-end (per plan
  # §"apply_profile_env value constraints": newlines must be preserved because the
  # sidecar is JSON). The decode uses NUL-terminated read instead of command
  # substitution to avoid bash's trailing-newline stripping.
  local key b64 value
  while IFS=$'\t' read -r key b64; do
    [[ -n "$key" ]] || continue
    [[ "$key" =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]] || {
      cs::helpers::log "warning: skipped unsafe env key from compose sidecar: $key"
      continue
    }
    IFS= read -r -d '' value < <(printf '%s' "$b64" | base64 -d && printf '\0') \
      || cs::helpers::die 3 "compose sidecar contains an undecodable env value." \
        "Failed to base64-decode the value for \"$key\" in $sidecar." \
        "  Re-run claude-session doctor and inspect the merged settings layers."
    printf -v "$key" '%s' "$value"
    export "${key?}"
  done < <(jq -r '.env | to_entries[]? | "\(.key)\t\((.value | tostring) | @base64)"' "$sidecar")
}
