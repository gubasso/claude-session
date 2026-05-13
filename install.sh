#!/usr/bin/env bash
set -euo pipefail
shopt -s inherit_errexit 2>/dev/null || true

completions_only=0
if [[ "${1:-}" == "--completions-only" ]]; then
  completions_only=1
fi

repo_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
user_mode=0
if [[ $EUID -ne 0 && -z "${PREFIX:-}" ]]; then
  user_mode=1
fi

if [[ $user_mode -eq 1 ]]; then
  bin_dir="$HOME/.local/bin"
  lib_dir="$HOME/.local/lib/claude-session"
  data_dir="${XDG_DATA_HOME:-$HOME/.local/share}"
  completion_dir="$data_dir/bash-completion/completions"
  state_dir="${XDG_STATE_HOME:-$HOME/.local/state}/claude-session"
  cache_dir="${XDG_CACHE_HOME:-$HOME/.cache}/claude-session"
else
  prefix=${PREFIX:-/usr/local}
  bin_dir="$prefix/bin"
  lib_dir="$prefix/lib/claude-session"
  if command -v pkg-config >/dev/null 2>&1 && pkg-config --exists bash-completion 2>/dev/null; then
    completion_dir=$(pkg-config --variable=completionsdir bash-completion)
  else
    completion_dir="$prefix/share/bash-completion/completions"
  fi
  state_dir="/var/lib/claude-session"
fi

manifest="$state_dir/install-manifest"
new_manifest="$manifest.tmp.$$"
mkdir -p "$state_dir"
: >"$new_manifest"

record() {
  printf '%s\n' "$1" >>"$new_manifest"
}

install_one() {
  local src=$1
  local dst=$2
  local mode=${3:-644}
  mkdir -p "$(dirname "$dst")"
  cp "$src" "$dst"
  chmod "$mode" "$dst"
  record "$dst"
}

if [[ $completions_only -eq 0 ]]; then
  mkdir -p "$lib_dir" "$lib_dir/commands" "$lib_dir/functions"
  # Record directories first; uninstall iterates the manifest in reverse,
  # so files come off first, then their subdirectories, then the parent.
  record "$lib_dir"
  record "$lib_dir/commands"
  record "$lib_dir/functions"
  install_one "$repo_dir/bin/claude-session" "$bin_dir/claude-session" 755
  for src in "$repo_dir/lib"/*.sh; do
    install_one "$src" "$lib_dir/$(basename "$src")" 644
  done
  for src in "$repo_dir/lib/commands"/*.sh; do
    install_one "$src" "$lib_dir/commands/$(basename "$src")" 644
  done
  for src in "$repo_dir/lib/functions"/*.sh; do
    install_one "$src" "$lib_dir/functions/$(basename "$src")" 644
  done
fi

install_one "$repo_dir/completions/claude-session.bash" "$completion_dir/claude-session.bash" 644

if [[ $completions_only -eq 1 && -f "$manifest" ]]; then
  # Merge: preserve every previously installed path that is not the
  # completions file we just rewrote, then append the fresh manifest line.
  while IFS= read -r old; do
    [[ -n "$old" ]] || continue
    [[ "$old" == "$completion_dir/claude-session.bash" ]] && continue
    grep -Fxq "$old" "$new_manifest" || printf '%s\n' "$old" >>"$new_manifest"
  done <"$manifest"
elif [[ -f "$manifest" ]]; then
  # Full install: any path in the previous manifest absent from the new one
  # is stale and should be removed.
  while IFS= read -r old; do
    [[ -n "$old" ]] || continue
    if ! grep -Fxq "$old" "$new_manifest"; then
      if [[ -f "$old" || -L "$old" ]]; then
        rm -f "$old"
      elif [[ -d "$old" ]]; then
        rmdir "$old" 2>/dev/null || true
      fi
    fi
  done <"$manifest"
fi
mv -f "$new_manifest" "$manifest"

if [[ $user_mode -eq 1 ]]; then
  # Pre-create the persistent runtime-settings cache dir on the host so
  # devcontainer bind-mounts of `~/.cache/claude-session` succeed without
  # the user running mkdir manually. Kept outside the install manifest;
  # uninstall.sh removes the cache dir explicitly (regeneratable state).
  mkdir -p "$cache_dir"
fi

if [[ $user_mode -eq 1 && ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
  # shellcheck disable=SC2016  # literal $HOME and $PATH for the user to copy/paste
  printf '[claude-session] warning: add this to your shell rc: export PATH="$HOME/.local/bin:$PATH"\n' >&2
fi
