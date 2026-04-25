#!/usr/bin/env bash
set -euo pipefail
shopt -s inherit_errexit 2>/dev/null || true

user_mode=0
if [[ $EUID -ne 0 && -z "${PREFIX:-}" ]]; then
  user_mode=1
fi

if [[ $user_mode -eq 1 ]]; then
  state_dir="${XDG_STATE_HOME:-$HOME/.local/state}/claude-session"
else
  state_dir="/var/lib/claude-session"
fi
manifest="$state_dir/install-manifest"

if [[ ! -f "$manifest" ]]; then
  printf '[claude-session] warning: install manifest missing at %s\n' "$manifest" >&2
  exit 1
fi

mapfile -t paths <"$manifest"
for ((i = ${#paths[@]} - 1; i >= 0; i--)); do
  path=${paths[$i]}
  [[ -n "$path" ]] || continue
  if [[ "$path" == "$state_dir" && -d "$state_dir/sessions" ]]; then
    continue
  fi
  if [[ -f "$path" || -L "$path" ]]; then
    rm -f "$path"
  elif [[ -d "$path" ]]; then
    rmdir "$path" 2>/dev/null || true
  fi
done
rm -f "$manifest"
