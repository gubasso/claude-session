# shellcheck shell=bash
#
# Shared install/uninstall UX library.
#
# This file is a project-neutral helper set sourced by the generated
# install.sh and uninstall.sh. It owns the presentation and error-handling
# standard only: TTY-aware color, leveled progress messages, a contextual
# ERR trap, preflight guards, and generic path-pruning helpers. It holds NO
# project-specific paths or payload — those live in install.sh / uninstall.sh.
#
# Contract shared by every generated installer:
#   - stdout carries machine-readable data; stderr carries progress and errors.
#   - color activates only on a TTY with NO_COLOR unset.
#   - exit codes: 0 success, 1 handled failure, 130 SIGINT, 143 SIGTERM.

installer_current_step=""
installer_current_hint=""
installer_quiet="${INSTALLER_QUIET:-0}"
installer_verbose="${INSTALLER_VERBOSE:-0}"
installer_color_step=""
installer_color_success=""
installer_color_warning=""
installer_color_error=""
installer_color_detail=""
# shellcheck disable=SC2034 # Shared color token reserved for caller-facing UI text.
installer_color_bold=""
installer_color_reset=""

installer_color_enabled() {
  [[ -t 2 && -z ${NO_COLOR:-} ]]
}

installer_ui_init() {
  if installer_color_enabled; then
    installer_color_step=$'\033[1;34m'
    installer_color_success=$'\033[1;32m'
    installer_color_warning=$'\033[1;33m'
    installer_color_error=$'\033[1;31m'
    installer_color_detail=$'\033[2m'
    # shellcheck disable=SC2034 # Shared color token reserved for caller-facing UI text.
    installer_color_bold=$'\033[1m'
    installer_color_reset=$'\033[0m'
  else
    installer_color_step=""
    installer_color_success=""
    installer_color_warning=""
    installer_color_error=""
    installer_color_detail=""
    # shellcheck disable=SC2034 # Shared color token reserved for caller-facing UI text.
    installer_color_bold=""
    installer_color_reset=""
  fi
}

installer_note() {
  ((installer_quiet == 1)) && return 0
  printf '%s\n' "$*" >&2
}

installer_detail() {
  ((installer_quiet == 1)) && return 0
  ((installer_verbose == 1)) || return 0
  printf '    %s%s%s\n' "$installer_color_detail" "$*" "$installer_color_reset" >&2
}

installer_step() {
  ((installer_quiet == 1)) && return 0
  printf '%s==>%s %s\n' "$installer_color_step" "$installer_color_reset" "$*" >&2
}

installer_ok() {
  ((installer_quiet == 1)) && return 0
  printf '%sok%s %s\n' "$installer_color_success" "$installer_color_reset" "$*" >&2
}

installer_warn() {
  printf '%swarning:%s %s\n' "$installer_color_warning" "$installer_color_reset" "$*" >&2
}

installer_error() {
  printf '%serror:%s %s\n' "$installer_color_error" "$installer_color_reset" "$*" >&2
}

installer_die() {
  installer_error "$*"
  exit 1
}

installer_err_trap() {
  local status="$1"
  local line="$2"
  local command="$3"
  local hint="${installer_current_hint:-Check the command output above, fix the failing condition, and retry.}"

  if [[ -n $installer_current_step ]]; then
    installer_error "failed during ${installer_current_step}"
  else
    installer_error "command failed"
  fi
  printf '    line: %s\n' "$line" >&2
  printf '    command: %s\n' "$command" >&2
  printf '    status: %s\n' "$status" >&2
  printf '    hint: %s\n' "$hint" >&2
  exit "$status"
}

installer_set_step() {
  installer_current_step="$1"
  installer_current_hint="${2:-Check the command output above, fix the failing condition, and retry.}"
}

installer_require_command() {
  local name="$1"
  local required_or_optional="$2"
  local purpose="$3"

  if command -v "$name" >/dev/null 2>&1; then
    installer_detail "found $name for $purpose"
    return 0
  fi

  if [[ $required_or_optional == "required" ]]; then
    installer_die "missing required command '$name' for $purpose; install it and retry"
  fi

  installer_warn "optional command '$name' not found; $purpose"
  return 0
}

installer_require_home() {
  if [[ -z ${HOME:-} ]]; then
    installer_die "HOME is not set; set HOME or run from a normal user environment"
  fi
}

installer_require_writable_dir() {
  local dir="$1"
  local label="$2"

  install -d "$dir"
  if [[ ! -d $dir || ! -w $dir ]]; then
    installer_die "$label is not writable at $dir; fix permissions or choose a different PREFIX/XDG path"
  fi
}

installer_require_parent_writable() {
  local path="$1"
  local label="$2"
  local parent

  parent="$(dirname "$path")"
  install -d "$parent"
  if [[ ! -d $parent || ! -w $parent ]]; then
    installer_die "$label parent is not writable at $parent; fix permissions or choose a different PREFIX/XDG path"
  fi
}

# Generic path helpers shared by the manifest-driven install/uninstall pair.
# The project-specific manifest allowlist (valid_manifest_path) lives in the
# generated install.sh / uninstall.sh, because only those files know the real
# install roots.
path_under() {
  local path="$1"
  local root="$2"

  [[ $path == "$root" || $path == "$root"/* ]]
}

rmdir_empty() {
  local dir="$1"
  rmdir -- "$dir" 2>/dev/null || true
  return 0
}

prune_empty_tree() {
  local root="$1"
  local dir

  [[ -d $root ]] || return 0
  while IFS= read -r dir; do
    rmdir_empty "$dir"
  done < <(find "$root" -depth -type d -print)

  return 0
}

prune_manifest_dir() {
  local path="$1"
  local root="$2"
  local dir

  path_under "$path" "$root" || return 0
  dir="$(dirname "$path")"
  while [[ $dir != "$root" && $dir == "$root"/* ]]; do
    rmdir_empty "$dir"
    dir="$(dirname "$dir")"
  done

  return 0
}
