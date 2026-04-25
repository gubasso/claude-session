# shellcheck shell=bash
: 'desc: Parse global flags, load configuration, resolve profile, and dispatch.'

CS_VERSION="0.1.0"

cs::main() {
  local -a args=("$@")
  local -a rest=()
  local sub=""
  local help_requested=0
  local i=0

  CS_CLI_PROFILE=""
  CS_CLI_CONFIG=""
  CS_GLOBAL_DRY_RUN=0
  export CS_CLI_PROFILE CS_CLI_CONFIG CS_GLOBAL_DRY_RUN

  while [[ $i -lt ${#args[@]} ]]; do
    case "${args[$i]}" in
      --profile)
        [[ $((i + 1)) -lt ${#args[@]} ]] || cs::helpers::die 2 "missing value for --profile." "--profile requires a profile name." "  claude-session --profile default doctor"
        CS_CLI_PROFILE=${args[$((i + 1))]}
        i=$((i + 2))
        ;;
      --config)
        [[ $((i + 1)) -lt ${#args[@]} ]] || cs::helpers::die 2 "missing value for --config." "--config requires a path." "  claude-session --config /path/to/config.env doctor"
        CS_CLI_CONFIG=${args[$((i + 1))]}
        i=$((i + 2))
        ;;
      --verbose)
        CLAUDE_SESSION_VERBOSE=1
        export CLAUDE_SESSION_VERBOSE
        i=$((i + 1))
        ;;
      --dry-run)
        CS_GLOBAL_DRY_RUN=1
        i=$((i + 1))
        ;;
      --help | -h)
        help_requested=1
        i=$((i + 1))
        ;;
      --version)
        printf 'claude-session %s\n' "$CS_VERSION"
        return 0
        ;;
      --)
        rest=("${args[@]:$((i + 1))}")
        sub=run
        break
        ;;
      -*)
        rest=("${args[@]:$i}")
        sub=run
        break
        ;;
      *)
        if cs::loader::is_command "${args[$i]}"; then
          sub=${args[$i]}
          rest=("${args[@]:$((i + 1))}")
        else
          sub=run
          rest=("${args[@]:$i}")
        fi
        break
        ;;
    esac
  done

  if [[ -z "$sub" ]]; then
    if [[ $help_requested -eq 1 ]]; then
      sub=usage
    else
      sub=run
    fi
  fi

  if [[ $help_requested -eq 1 && "$sub" != "usage" ]]; then
    rest=("--help" "${rest[@]}")
  fi

  if [[ "$sub" == "usage" || "${rest[0]:-}" == "--help" || "${rest[0]:-}" == "-h" ]]; then
    cs::loader::dispatch "$sub" "${rest[@]}"
    return $?
  fi

  cs::helpers::source_fn load_config
  cs::fn::load_config
  if [[ -n "$CS_CLI_PROFILE" ]]; then
    CLAUDE_SESSION_PROFILE=$CS_CLI_PROFILE
    export CLAUDE_SESSION_PROFILE
  fi
  if [[ $CS_GLOBAL_DRY_RUN -eq 1 ]]; then
    CLAUDE_SESSION_DRY_RUN=1
    export CLAUDE_SESSION_DRY_RUN
  fi

  # Doctor must be able to report on a broken profile rather than aborting
  # in the global resolver. It runs resolve_profile itself with error
  # capture, so we skip the eager resolution here.
  if [[ "$sub" != "doctor" ]]; then
    cs::helpers::source_fn resolve_profile
    cs::fn::resolve_profile
  fi

  cs::loader::dispatch "$sub" "${rest[@]}"
}
