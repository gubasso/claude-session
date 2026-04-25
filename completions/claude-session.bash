# shellcheck shell=bash
# bash completion for claude-session

_claude_session() {
  local cur prev words cword
  COMPREPLY=()
  cur=${COMP_WORDS[COMP_CWORD]}
  prev=${COMP_WORDS[COMP_CWORD - 1]}
  words=("${COMP_WORDS[@]}")
  cword=$COMP_CWORD

  local commands="run doctor usage config profile session"
  local globals="--profile --config --verbose --dry-run --help --version"

  if [[ $cword -eq 1 ]]; then
    mapfile -t COMPREPLY < <(compgen -W "$globals $commands" -- "$cur")
    return 0
  fi

  case "${words[1]}" in
    config)
      mapfile -t COMPREPLY < <(compgen -W "show path edit --help --verbose" -- "$cur")
      ;;
    profile)
      mapfile -t COMPREPLY < <(compgen -W "list show --help --verbose" -- "$cur")
      ;;
    session)
      if [[ "$prev" == "session" ]]; then
        mapfile -t COMPREPLY < <(compgen -W "list clean --help" -- "$cur")
      else
        mapfile -t COMPREPLY < <(compgen -W "--absolute --no-header --older-than --dry-run --yes --help" -- "$cur")
      fi
      ;;
    run)
      mapfile -t COMPREPLY < <(compgen -W "--profile --dry-run --help --" -- "$cur")
      ;;
    doctor)
      mapfile -t COMPREPLY < <(compgen -W "--verbose --help" -- "$cur")
      ;;
    *)
      mapfile -t COMPREPLY < <(compgen -W "$globals $commands" -- "$cur")
      ;;
  esac
}

complete -F _claude_session claude-session
