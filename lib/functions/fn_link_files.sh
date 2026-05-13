# shellcheck shell=bash
: 'desc: Link shared Claude files and directories into a session directory.'

cs::fn::link_files() {
  local session_dir=$1
  local shared_dir=$2
  local link_files=${CLAUDE_SESSION_LINK_FILES:-settings.local.json:keybindings.json:CLAUDE.md}
  local home_link_files=${CLAUDE_SESSION_HOME_LINK_FILES:-.claude.json}
  local link_dirs=${CLAUDE_SESSION_LINK_DIRS:-skills:agents:rules:commands:hooks:plugins}
  local -a items=()
  local item=""

  cs::helpers::split_colon "$link_files" items
  for item in "${items[@]}"; do
    [[ -n "$item" ]] || continue
    if [[ -e "$shared_dir/$item" ]]; then
      mkdir -p "$(dirname "$session_dir/$item")"
      ln -sfn "$shared_dir/$item" "$session_dir/$item"
    fi
  done

  items=()
  cs::helpers::split_colon "$home_link_files" items
  for item in "${items[@]}"; do
    [[ -n "$item" ]] || continue
    # Only ensure the parent directories exist and (re)create the
    # session-side symlink here. Seeding $HOME/<item> with "{}" when it is
    # missing is delegated to the locked critical section in
    # __run_auto_trust_cwd so concurrent first-launch terminals never race
    # each other's "{}" writes (Stage 5 finding 1) and so --dry-run does
    # not mutate $HOME/ files (Stage 5 finding 3).
    mkdir -p "$(dirname "$HOME/$item")" "$(dirname "$session_dir/$item")"
    if [[ -e "$session_dir/$item" || -L "$session_dir/$item" ]]; then
      rm -f "$session_dir/$item"
    fi
    ln -sfn "$HOME/$item" "$session_dir/$item"
  done

  items=()
  cs::helpers::split_colon "$link_dirs" items
  for item in "${items[@]}"; do
    [[ -n "$item" ]] || continue
    if [[ -d "$shared_dir/$item" ]]; then
      if [[ -e "$session_dir/$item" || -L "$session_dir/$item" ]]; then
        rm -rf -- "${session_dir:?}/${item:?}"
      fi
      ln -sfn "$shared_dir/$item" "$session_dir/$item"
    fi
  done
}
