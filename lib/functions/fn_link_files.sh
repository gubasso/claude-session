# shellcheck shell=bash
: 'desc: Link shared Claude files and directories into a session directory.'

cs::fn::link_files() {
  local session_dir=$1
  local shared_dir=$2
  local link_files=${CLAUDE_SESSION_LINK_FILES:-settings.local.json:keybindings.json:CLAUDE.md}
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
