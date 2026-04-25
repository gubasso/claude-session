# shellcheck shell=bash
: 'desc: Render active and stale session directories as a shared table.'

__session_status() {
  local id=$1
  local dev=""
  case "$id" in
    pts-* | tty*)
      dev="/dev/${id//-//}"
      [[ -e "$dev" ]] && printf 'active\n' || printf 'stale\n'
      ;;
    pid-*)
      kill -0 "${id#pid-}" 2>/dev/null && printf 'active\n' || printf 'stale\n'
      ;;
    *)
      printf 'stale\n'
      ;;
  esac
}

__dir_size() {
  local dir=$1
  local bytes
  bytes=$(du -sB1 "$dir" 2>/dev/null | awk '{print $1}') || bytes=0
  if command -v numfmt >/dev/null 2>&1; then
    numfmt --to=iec --suffix=iB "$bytes"
  else
    printf '%s B\n' "$bytes"
  fi
}

cs::fn::session_inventory() {
  local absolute=0
  local no_header=0
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --absolute) absolute=1 ;;
      --no-header) no_header=1 ;;
      *) cs::helpers::die 2 "unknown session inventory flag \"$1\"." "The inventory renderer only accepts --absolute and --no-header." "  claude-session session list --help" ;;
    esac
    shift
  done

  cs::helpers::source_fn session_dir
  local root
  root=$(cs::fn::session_dir)
  local root_source
  root_source=$(cs::fn::session_root_source_for "$root")
  local parent="$root/sessions"
  local source_label="XDG_STATE_HOME"
  [[ "$root_source" == "runtime" ]] && source_label="XDG_RUNTIME_DIR"

  if [[ $no_header -eq 0 ]]; then
    printf '%s/  (%s)\n' "$parent" "$source_label"
    printf '# %-20s %-8s %-8s %s\n' "terminal_id" "status" "size" "mtime"
  fi

  local dir=""
  local id=""
  local status=""
  local size=""
  local mtime=""
  local display=""
  local tmp
  tmp=$(mktemp)
  shopt -u failglob
  shopt -s nullglob
  for dir in "$parent"/*; do
    [[ -d "$dir" ]] || continue
    id=$(basename "$dir")
    status=$(__session_status "$id")
    size=$(__dir_size "$dir")
    mtime=$(date -u -r "$dir" '+%FT%TZ' 2>/dev/null || printf '1970-01-01T00:00:00Z')
    display=$id
    [[ $absolute -eq 1 ]] && display=$dir
    printf '%s\t%s\t%s\t%s\n' "$mtime" "$display" "$status" "$size" >>"$tmp"
  done
  sort -r "$tmp" | while IFS=$'\t' read -r mtime display status size; do
    [[ -n "$display" ]] || continue
    printf '%-20s %-8s %-8s %s\n' "$display" "$status" "$size" "$mtime"
  done
  rm -f "$tmp"
}
