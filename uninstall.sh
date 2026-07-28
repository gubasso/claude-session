#!/usr/bin/env bash
set -eEuo pipefail
shopt -s inherit_errexit 2>/dev/null || true

# Native-toolchain uninstaller for a Rust project: wraps `cargo uninstall`
# behind the shared verbose UX. Idempotent — `cargo uninstall` exits 101 when
# the package is not installed, so this script first asks `cargo install --list`
# (which exits 0 even for a nonexistent root) and reports an absent package as a
# note rather than a failure. A real uninstall error still fails loudly.
#
# `cargo uninstall` takes the PACKAGE name from Cargo.toml, not a path and not a
# binary name; `cargo install --path` registers under that same package name.
#
# Environment:
#   PREFIX              when set, cargo uninstalls from $PREFIX/bin (cargo --root)
#   CARGO_INSTALL_ROOT  honored by cargo when PREFIX is unset
#   INSTALLER_QUIET=1   suppress progress output
#   INSTALLER_VERBOSE=1 print detail
#   NO_COLOR            disable color

case "${BASH_SOURCE[0]}" in
  */*) _self_dir="$(cd -P "${BASH_SOURCE[0]%/*}" && pwd)" ;;
  *) _self_dir="$(pwd)" ;;
esac
# shellcheck source=/dev/null
. "$_self_dir/install-common.sh"
installer_ui_init
installer_require_home

# ============================================================================
# PROJECT CONFIGURATION — set the installed package name (from Cargo.toml).
# ============================================================================
project_name="claude-session"
package_name="$project_name"
# ============================================================================

# Mirror install.sh: only pin --root when the caller asked for one, so a
# project-level `install.root` config still resolves the same way it did at
# install time.
root_args=()
if [[ -n ${PREFIX:-} ]]; then
  root_args+=(--root "$PREFIX")
  install_root="$PREFIX"
else
  install_root="${CARGO_INSTALL_ROOT:-${CARGO_HOME:-$HOME/.cargo}}"
fi

trap 'installer_err_trap "$?" "$LINENO" "$BASH_COMMAND"' ERR

installer_set_step "preflight" "A Rust toolchain (cargo) must be installed and on PATH."
installer_step "Preflight"
installer_require_command cargo required "uninstalling the crate"
installer_detail "package: $package_name"
installer_detail "install root: $install_root"
installer_ok "Preflight"

installer_set_step "check install record" \
  "Could not read cargo's install list; check the cargo output above."
installer_step "Check install record (cargo install --list)"
# Capture first, then match: piping straight into `grep -q` lets grep exit
# early, which under `set -o pipefail` turns cargo's SIGPIPE into a false
# negative.
installed=0
install_list="$(cargo install --list "${root_args[@]+"${root_args[@]}"}")"
if grep -Eq "^${package_name} v" <<<"$install_list"; then
  installed=1
fi
installer_detail "installed: $installed"
installer_ok "Check install record"

if ((installed == 0)); then
  installer_note "'$package_name' is not installed under $install_root; nothing to do"
  printf 'uninstalled %s via cargo\n' "$project_name"
  exit 0
fi

installer_set_step "uninstall crate" \
  "Check the cargo output above; the package is registered but could not be removed."
installer_step "Uninstall crate (cargo uninstall)"
cargo uninstall "$package_name" "${root_args[@]+"${root_args[@]}"}"
installer_ok "Uninstall crate"

printf 'uninstalled %s via cargo\n' "$project_name"
exit 0
