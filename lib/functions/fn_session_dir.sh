# shellcheck shell=bash
: 'desc: Resolve and validate the secure XDG session root.'

__session_root_validate() {
  local root=$1
  local source=$2
  if [[ -e "$root" && ( ! -d "$root" || -L "$root" ) ]]; then
    return 1
  fi
  mkdir -p "$root" "$root/sessions" || return 1
  if [[ -L "$root" || ! -d "$root" ]]; then
    return 1
  fi
  local owner
  owner=$(stat -c '%u' "$root" 2>/dev/null) || return 1
  [[ "$owner" == "$(id -u)" ]] || return 1
  chmod 700 "$root" "$root/sessions" || return 1
  CS_SESSION_ROOT_SOURCE=$source
  export CS_SESSION_ROOT_SOURCE
  printf '%s\n' "$root"
}

cs::fn::session_dir() {
  local runtime_candidate=""
  local state_candidate=""
  local root=""

  if [[ -n "${XDG_RUNTIME_DIR:-}" ]]; then
    runtime_candidate="$XDG_RUNTIME_DIR/claude-session"
    if root=$(__session_root_validate "$runtime_candidate" runtime); then
      printf '%s\n' "$root"
      return 0
    fi
  fi

  if [[ -z "${HOME:-}" ]]; then
    cs::helpers::die 5 "secure session-dir validation failed." "HOME is unset and XDG_RUNTIME_DIR did not resolve to a usable directory." "  Set XDG_RUNTIME_DIR or HOME before running claude-session." "claude-session doctor"
  fi
  state_candidate="${XDG_STATE_HOME:-$HOME/.local/state}/claude-session"
  if root=$(__session_root_validate "$state_candidate" state); then
    printf '%s\n' "$root"
    return 0
  fi

  cs::helpers::die 5 "secure session-dir validation failed." "Neither XDG_RUNTIME_DIR nor the XDG_STATE_HOME fallback resolved a usable, owner-correct, non-symlink directory." "  Fix ownership and permissions on the XDG runtime/state directory." "claude-session doctor"
}

cs::fn::session_root_source_for() {
  local root=$1
  if [[ -n "${XDG_RUNTIME_DIR:-}" && "$root" == "$XDG_RUNTIME_DIR/claude-session" ]]; then
    printf 'runtime\n'
  else
    printf 'state\n'
  fi
}
