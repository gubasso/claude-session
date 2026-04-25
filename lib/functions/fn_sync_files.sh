# shellcheck shell=bash
: 'desc: Copy sync-classified Claude files in and back out atomically.'

cs::fn::sync_files() {
  local mode=$1
  local session_dir=$2
  local shared_dir=$3
  local sync_files=${CLAUDE_SESSION_SYNC_FILES:-.credentials.json}
  local -a items=()
  local item=""
  cs::helpers::split_colon "$sync_files" items

  case "$mode" in
    in)
      for item in "${items[@]}"; do
        [[ -n "$item" ]] || continue
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
          src="$session_dir/$item"
          dst="$shared_dir/$item"
          [[ -f "$src" ]] || continue
          mkdir -p "$(dirname "$dst")"
          src_m=$(stat -c '%Y' "$src" 2>/dev/null || printf '0')
          dst_m=$(stat -c '%Y' "$dst" 2>/dev/null || printf '0')
          if [[ ! -f "$dst" || $src_m -gt $dst_m ]]; then
            tmp="$dst.tmp.$$"
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
