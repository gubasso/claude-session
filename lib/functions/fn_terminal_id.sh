# shellcheck shell=bash
: 'desc: Derive a stable terminal identifier from tty with a PID fallback.'

cs::fn::terminal_id() {
  local t
  t=$(tty 2>/dev/null) || {
    printf 'pid-%s\n' "$$"
    return 0
  }
  if [[ "$t" != /dev/* ]]; then
    printf 'pid-%s\n' "$$"
    return 0
  fi
  t=${t#/dev/}
  printf '%s\n' "${t//\//-}"
}
