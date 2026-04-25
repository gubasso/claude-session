# shellcheck shell=bash
: 'desc: List and clean claude-session session directories.'

__session_help() {
  cat <<'EOF'
USAGE:
  claude-session session list [--absolute] [--no-header]
  claude-session session clean [--older-than <duration>] [--dry-run] [--yes]
EOF
}

__parse_duration() {
  local dur=$1
  [[ "$dur" =~ ^([0-9]+)([smhdw])$ ]] || return 1
  local n=${BASH_REMATCH[1]}
  local u=${BASH_REMATCH[2]}
  case "$u" in
    s) printf '%s\n' "$n" ;;
    m) printf '%s\n' "$((n * 60))" ;;
    h) printf '%s\n' "$((n * 3600))" ;;
    d) printf '%s\n' "$((n * 86400))" ;;
    w) printf '%s\n' "$((n * 604800))" ;;
  esac
}

__session_list() {
  cs::helpers::source_fn session_inventory
  cs::fn::session_inventory "$@"
}

__session_clean() {
  local older_than=""
  local dry_run=${CLAUDE_SESSION_DRY_RUN:-0}
  local yes=0
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --older-than)
        older_than=${2:-}
        [[ -n "$older_than" ]] || cs::helpers::die 2 "missing duration." "--older-than requires a duration like 7d." "  claude-session session clean --older-than 7d --dry-run"
        shift 2
        ;;
      --dry-run)
        dry_run=1
        shift
        ;;
      --yes)
        yes=1
        shift
        ;;
      --help | -h)
        __session_help
        return 0
        ;;
      *) cs::helpers::die 2 "unknown session clean flag \"$1\"." "The session clean command did not understand \"$1\"." "  claude-session session clean --help" ;;
    esac
  done

  local age_seconds=""
  if [[ -n "$older_than" ]]; then
    age_seconds=$(__parse_duration "$older_than") || cs::helpers::die 2 "invalid duration \"$older_than\"." "Use N[smhdw], for example 30s, 15m, 2h, 7d, 4w." "  claude-session session clean --older-than 7d --dry-run"
  fi

  cs::helpers::source_fn session_dir
  local root
  root=$(cs::fn::session_dir)
  local parent="$root/sessions"
  local now
  now=$(date +%s)
  local -a candidates=()
  local dir=""
  shopt -u failglob
  shopt -s nullglob
  for dir in "$parent"/*; do
    [[ -d "$dir" ]] || continue
    local id
    id=$(basename "$dir")
    local status
    cs::helpers::source_fn session_inventory
    status=$(__session_status "$id")
    [[ "$status" == "stale" ]] || continue
    if [[ -n "$age_seconds" ]]; then
      local mtime
      mtime=$(stat -c '%Y' "$dir")
      [[ $((now - mtime)) -gt $age_seconds ]] || continue
    fi
    candidates+=("$dir")
  done

  printf 'Would remove %s stale session directories under\n%s:\n' "${#candidates[@]}" "$parent"
  local cand_id cand_date cand_size
  for dir in "${candidates[@]}"; do
    cand_id=$(basename "$dir")
    cand_date=$(date -u -r "$dir" '+%Y-%m-%d' 2>/dev/null || printf 'unknown')
    cand_size=$(du -sh "$dir" 2>/dev/null | awk '{print $1}')
    [[ -n "$cand_size" ]] || cand_size="?"
    printf '  %-12s (%s, %s)\n' "$cand_id" "$cand_date" "$cand_size"
  done
  if [[ $dry_run -eq 1 || ${#candidates[@]} -eq 0 ]]; then
    printf '\nNext:\n  Remove them:  claude-session session clean --yes\n'
    return 0
  fi
  if [[ $yes -ne 1 && ! -t 0 ]]; then
    cs::helpers::die 2 "confirmation required." "stdin is not a TTY, so session clean requires --yes." "  claude-session session clean --yes"
  fi
  if [[ $yes -ne 1 ]]; then
    printf 'Remove these directories? [y/N] ' >&2
    local answer=""
    read -r answer
    [[ "$answer" == "y" || "$answer" == "Y" ]] || return 0
  fi
  local failures=0
  local -a failed=()
  for dir in "${candidates[@]}"; do
    if ! rm -rf -- "$dir" 2>/dev/null; then
      failures=$((failures + 1))
      failed+=("$dir")
    fi
  done
  if [[ $failures -ne 0 ]]; then
    cs::helpers::source_fn error
    local list=""
    local f
    for f in "${failed[@]}"; do
      list+="    $f"$'\n'
    done
    cs::fn::error \
      "session clean removed $((${#candidates[@]} - failures))/${#candidates[@]} directories." \
      "$failures session director$( [[ $failures -eq 1 ]] && printf y || printf ies) could not be removed:"$'\n'"${list%$'\n'}" \
      "  Check permissions and re-run:  claude-session session clean --yes" \
      "claude-session doctor"
    return 1
  fi
}

cs::cmd::session() {
  local sub=${1:-}
  [[ $# -gt 0 ]] && shift
  case "$sub" in
    list) __session_list "$@" ;;
    clean) __session_clean "$@" ;;
    --help | -h | "") __session_help ;;
    *) cs::helpers::die 2 "unknown session subcommand \"$sub\"." "Expected list or clean." "  claude-session session --help" ;;
  esac
}
