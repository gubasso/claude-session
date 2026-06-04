# shellcheck shell=bash
: 'desc: Copy sync-classified Claude files in and back out atomically.'

# Reserved compose artifacts must never round-trip through the shared dir:
# settings.json and its sidecar are recomposed per-profile from versioned
# layers on every launch. Syncing them out then back in would replay one
# profile's composed settings/env over another profile's fresh composition,
# reintroducing the cross-profile leak. Guard regardless of how the caller
# configures CLAUDE_SESSION_SYNC_FILES.
#
# Items are interpolated as "$dir/$item", so path-decorated aliases like
# "./settings.json" or "foo/../settings.json" resolve to the same reserved
# file. Reject by basename, and reject any "../" traversal outright (it would
# also let an item escape the session/shared dirs entirely).
__sync_is_reserved() {
  local item=$1
  # Wrapping in slashes makes a ".." segment always appear as "/../",
  # so a single pattern catches leading, embedded, and trailing traversal.
  case "/$item/" in
    */../*) return 0 ;;
  esac
  case "${item##*/}" in
    settings.json | .claude-session-compose.json) return 0 ;;
    *) return 1 ;;
  esac
}

cs::fn::sync_files() {
  local mode=$1
  local session_dir=$2
  local shared_dir=$3
  local sync_files=${CLAUDE_SESSION_SYNC_FILES:-.credentials.json:mcp-needs-auth-cache.json}
  local -a items=()
  local item=""
  cs::helpers::split_colon "$sync_files" items

  case "$mode" in
    in)
      for item in "${items[@]}"; do
        [[ -n "$item" ]] || continue
        __sync_is_reserved "$item" && continue
        if [[ -f "$shared_dir/$item" ]]; then
          mkdir -p "$(dirname "$session_dir/$item")"
          cp -p "$shared_dir/$item" "$session_dir/$item"
        fi
      done
      ;;
    out)
      command -v flock >/dev/null 2>&1 || cs::helpers::die 3 "flock is required." "Cannot safely sync rewritten Claude files without flock." "  Install util-linux flock."
      mkdir -p "$shared_dir"
      local lock="$shared_dir/.claude-session.lock"
      (
        flock 9
        local src=""
        local dst=""
        local tmp=""
        local src_m=0
        local dst_m=0
        for item in "${items[@]}"; do
          [[ -n "$item" ]] || continue
          __sync_is_reserved "$item" && continue
          src="$session_dir/$item"
          dst="$shared_dir/$item"
          [[ -f "$src" ]] || continue
          mkdir -p "$(dirname "$dst")"
          tmp="$dst.tmp.$$"
          src_m=$(stat -c '%Y' "$src" 2>/dev/null || printf '0')
          dst_m=$(stat -c '%Y' "$dst" 2>/dev/null || printf '0')
          if [[ ! -f "$dst" || $src_m -gt $dst_m ]]; then
            cp -p "$src" "$tmp"
            mv -f "$tmp" "$dst"
          fi
        done
      ) 9>"$lock"
      ;;
    *)
      cs::helpers::die 2 "invalid sync mode \"$mode\"." "sync_files accepts only \"in\" or \"out\"." "  Report this as an internal bug."
      ;;
  esac
}
