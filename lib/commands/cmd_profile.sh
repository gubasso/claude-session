# shellcheck shell=bash
: 'desc: List profiles or show one profile dotenv file.'

__profile_help() {
  cat <<'EOF'
USAGE:
  claude-session profile list
  claude-session profile show <name> [--verbose]
EOF
}

__profile_list() {
  local config_dir
  config_dir=$(cs::helpers::config_dir_default)
  local dir="$config_dir/profiles"
  [[ -e "$dir" ]] || return 0
  [[ -r "$dir" ]] || cs::helpers::die 3 "profiles directory unreadable." "Cannot read $dir." "  chmod u+r \"$dir\""
  local active=${CLAUDE_SESSION_PROFILE:-default}
  local file=""
  while IFS= read -r file; do
    [[ -n "$file" ]] || continue
    local name
    name=$(basename "$file" .env)
    if [[ "$name" == "$active" ]]; then
      printf '* %s\n' "$name"
    else
      printf '  %s\n' "$name"
    fi
  done < <(find "$dir" -maxdepth 1 -type f -name '*.env' -print | sort)
}

__profile_show() {
  local name=$1
  local verbose=$2
  local config_dir
  config_dir=$(cs::helpers::config_dir_default)
  local file="$config_dir/profiles/$name.env"
  local overlay="$config_dir/profiles/$name.settings.json"
  [[ -f "$file" ]] || cs::helpers::die 6 "profile \"$name\" not found." "No file at $file." "  List available profiles:  claude-session profile list" "claude-session profile list"
  cs::helpers::source_fn load_config
  cs::fn::validate_dotenv "$file"
  printf '# profile: %s\n' "$name"
  printf '# source: %s\n' "$file"
  # Apply the validated dotenv in an isolated subshell so the parent
  # environment is not polluted, then dump only the keys the profile
  # actually defined. We use the same line-parser the loader uses so
  # values like `pass show <secret>` are not executed as commands.
  local profile_keys
  # shellcheck disable=SC2016  # script body for inner bash -c; vars expand in child
  if ! profile_keys=$(env -i HOME="$HOME" PATH="$PATH" \
      CS_PROFILE_FILE="$file" CS_LIB_DIR="${LIB_DIR:-}" \
      bash -c '
        # shellcheck source=/dev/null
        . "$CS_LIB_DIR/helpers.sh"
        # shellcheck source=/dev/null
        . "$CS_LIB_DIR/functions/fn_load_config.sh"
        cs::fn::__apply_dotenv "$CS_PROFILE_FILE"
        env -0 | tr "\0" "\n"
      ' 2>/dev/null); then
    cs::helpers::die 3 "profile \"$name\" failed to load." "Sourcing $file produced an error." "  Edit the file and re-run claude-session profile show $name."
  fi
  local kv key value
  while IFS= read -r kv; do
    [[ -n "$kv" ]] || continue
    [[ "$kv" == *=* ]] || continue
    key=${kv%%=*}
    value=${kv#*=}
    case "$key" in
      HOME | PATH | PWD | SHLVL | _ | OLDPWD | CS_PROFILE_FILE | CS_LIB_DIR) continue ;;
    esac
    value=$(cs::helpers::redact "$key" "$value" "$verbose")
    printf '%s=%s\n' "$key" "$value"
  done <<<"$profile_keys"
  if [[ -f "$overlay" ]]; then
    printf '# overlay: %s\n' "$overlay"
  fi
}

cs::cmd::profile() {
  local sub=${1:-}
  [[ $# -gt 0 ]] && shift
  case "$sub" in
    list)
      [[ $# -eq 0 ]] || cs::helpers::die 2 "profile list takes no arguments." "Unexpected argument: $1" "  claude-session profile list"
      __profile_list
      ;;
    show)
      local name=${1:-}
      [[ -n "$name" ]] || cs::helpers::die 2 "missing profile name." "profile show requires a profile name." "  claude-session profile show default"
      shift
      local verbose=0
      while [[ $# -gt 0 ]]; do
        case "$1" in
          --verbose) verbose=1 ;;
          --help | -h)
            __profile_help
            return 0
            ;;
          *) cs::helpers::die 2 "unknown profile flag \"$1\"." "The profile command did not understand \"$1\"." "  claude-session profile --help" ;;
        esac
        shift
      done
      __profile_show "$name" "$verbose"
      ;;
    --help | -h | "") __profile_help ;;
    *) cs::helpers::die 2 "unknown profile subcommand \"$sub\"." "Expected list or show." "  claude-session profile --help" ;;
  esac
}
