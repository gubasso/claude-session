# shellcheck shell=bash
: 'desc: Resolve, validate, and load the main dotenv configuration file.'

__dotenv_quote_balanced() {
  local value=$1
  local quote=""
  local char=""
  local escaped=0
  local i=0
  while [[ $i -lt ${#value} ]]; do
    char=${value:$i:1}
    if [[ $escaped -eq 1 ]]; then
      escaped=0
    elif [[ "$char" == "\\" ]]; then
      escaped=1
    elif [[ -z "$quote" && ( "$char" == "'" || "$char" == '"' ) ]]; then
      quote=$char
    elif [[ -n "$quote" && "$char" == "$quote" ]]; then
      quote=""
    fi
    i=$((i + 1))
  done
  [[ -z "$quote" ]]
}

cs::fn::validate_dotenv() {
  local file=$1
  local line=""
  local lineno=0
  local key=""
  local value=""
  while IFS= read -r line || [[ -n "$line" ]]; do
    lineno=$((lineno + 1))
    [[ -z "$line" || "$line" == \#* ]] && continue
    if [[ ! "$line" =~ ^[A-Za-z_][A-Za-z0-9_]*= ]]; then
      cs::helpers::die 3 "config syntax error." "$file:$lineno is not a KEY=VALUE assignment." "  Edit the file:  \${EDITOR:-vi} \"$file\"" "claude-session config edit"
    fi
    key=${line%%=*}
    value=${line#*=}
    # shellcheck disable=SC2016  # literal pattern fragments, not parameter expansions
    if [[ "$value" == *'$('* || "$value" == *';'* || "$value" == *'&&'* || "$value" == *'||'* || "$value" == *'|'* || "$value" == *'`'* || "$value" == *'<'* || "$value" == *'>'* ]]; then
      cs::helpers::die 3 "config syntax error." "$file:$lineno contains shell syntax in $key." "  Move complex logic into your shell rc or a hook script." "claude-session config edit"
    fi
    if ! __dotenv_quote_balanced "$value"; then
      cs::helpers::die 3 "config syntax error." "$file:$lineno has unbalanced quotes in $key." "  Fix the quoted value in:  $file" "claude-session config edit"
    fi
  done <"$file"
}

cs::fn::load_config() {
  local file
  file=$(cs::helpers::config_path)
  export CLAUDE_SESSION_CONFIG_FILE=$file
  export CLAUDE_SESSION_CONFIG_DIR
  CLAUDE_SESSION_CONFIG_DIR="$(dirname "$file")"

  [[ -e "$file" ]] || return 0
  [[ -r "$file" ]] || cs::helpers::die 3 "config file is unreadable." "Cannot read $file." "  chmod 600 \"$file\"" "claude-session config path"

  cs::fn::validate_dotenv "$file"

  local keys=(
    CLAUDE_SESSION_CONFIG_DIR
    CLAUDE_SESSION_SHARED_DIR
    CLAUDE_SESSION_PROFILE
    CLAUDE_SESSION_REAL_CLAUDE
    CLAUDE_SESSION_OAUTH_CMD
    CLAUDE_SESSION_POST_EXIT_CMD
    CLAUDE_SESSION_SYNC_FILES
    CLAUDE_SESSION_LINK_FILES
    CLAUDE_SESSION_LINK_DIRS
    CLAUDE_SESSION_VERBOSE
  )
  local key=""
  local -A had_env=()
  local -A env_val=()
  for key in "${keys[@]}"; do
    if [[ -v $key ]]; then
      had_env[$key]=1
      env_val[$key]=${!key}
    fi
  done

  cs::fn::__apply_dotenv "$file"

  for key in "${keys[@]}"; do
    if [[ "${had_env[$key]:-0}" == "1" ]]; then
      printf -v "$key" '%s' "${env_val[$key]}"
      export "${key?}"
    fi
  done
}

# Parse an already-validated dotenv file line-by-line and export each
# assignment. Avoids `source`-ing the file so values that contain spaces
# (e.g. CLAUDE_SESSION_OAUTH_CMD=pass show ...) are treated as plain
# string values rather than as KEY=word command-prefix assignments.
cs::fn::__apply_dotenv() {
  local file=$1
  local line=""
  local k=""
  local v=""
  while IFS= read -r line || [[ -n "$line" ]]; do
    [[ -z "$line" || "$line" == \#* ]] && continue
    [[ "$line" =~ ^[A-Za-z_][A-Za-z0-9_]*= ]] || continue
    k=${line%%=*}
    v=${line#*=}
    # Strip a single layer of matching surrounding single or double quotes.
    if [[ ${#v} -ge 2 ]]; then
      if [[ "${v:0:1}" == "\"" && "${v: -1}" == "\"" ]]; then
        v=${v:1:${#v}-2}
        v=${v//\\\"/\"}
        v=${v//\\\\/\\}
      elif [[ "${v:0:1}" == "'" && "${v: -1}" == "'" ]]; then
        v=${v:1:${#v}-2}
      fi
    fi
    printf -v "$k" '%s' "$v"
    export "${k?}"
  done <"$file"
}
