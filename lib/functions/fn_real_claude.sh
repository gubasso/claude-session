# shellcheck shell=bash
: 'desc: Discover the real claude binary without recursing into the wrapper.'

__is_wrapper_path() {
  local candidate=$1
  local candidate_real=""
  local wrapper_real=""
  candidate_real=$(readlink -f "$candidate" 2>/dev/null || printf '%s\n' "$candidate")
  wrapper_real=$(readlink -f "${CS_WRAPPER_PATH:-$0}" 2>/dev/null || printf '%s\n' "${CS_WRAPPER_PATH:-$0}")
  [[ "$candidate_real" == "$wrapper_real" ]]
}

cs::fn::real_claude() {
  local candidate=""
  if [[ -n "${CLAUDE_SESSION_REAL_CLAUDE:-}" ]]; then
    candidate=$CLAUDE_SESSION_REAL_CLAUDE
    if [[ -x "$candidate" ]] && ! __is_wrapper_path "$candidate"; then
      printf '%s\n' "$candidate"
      return 0
    fi
    cs::helpers::die 4 "real claude binary not found." "CLAUDE_SESSION_REAL_CLAUDE is not executable or resolves back to this wrapper: $candidate" "  Set CLAUDE_SESSION_REAL_CLAUDE=/path/to/claude" "claude-session doctor"
  fi

  local versions="$HOME/.local/share/claude/versions"
  if [[ -d "$versions" ]]; then
    # docs/architecture.md: pick the highest-sort-V entry that is
    # executable. Native installs may store the binary directly as
    # `versions/<ver>` (the file IS the version-named binary, per the
    # doctor example output) OR as `versions/<ver>/claude`. Try both.
    local entry=""
    while IFS= read -r entry; do
      candidate=$entry
      if [[ -d "$candidate" && -x "$candidate/claude" ]]; then
        candidate="$candidate/claude"
      fi
      [[ -f "$candidate" && -x "$candidate" ]] || continue
      if ! __is_wrapper_path "$candidate"; then
        printf '%s\n' "$candidate"
        return 0
      fi
    done < <(find "$versions" -mindepth 1 -maxdepth 1 -printf '%p\n' | sort -Vr)
  fi

  local dir=""
  local -a path_entries=()
  IFS=: read -r -a path_entries <<<"${PATH:-}"
  for dir in "${path_entries[@]}"; do
    [[ -n "$dir" ]] || continue
    candidate="$dir/claude"
    [[ -x "$candidate" ]] || continue
    if ! __is_wrapper_path "$candidate"; then
      printf '%s\n' "$candidate"
      return 0
    fi
  done

  cs::helpers::die 4 "real claude binary not found." "Searched \$HOME/.local/share/claude/versions/ and PATH; no executable named \"claude\" that does not resolve to this wrapper." "  Install Claude Code and run:  claude login"$'\n'"  Or set an override:       CLAUDE_SESSION_REAL_CLAUDE=/path/to/claude" "claude-session doctor"
}
