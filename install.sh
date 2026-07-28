#!/usr/bin/env bash
set -eEuo pipefail
shopt -s inherit_errexit 2>/dev/null || true

# Native-toolchain installer for a Rust project. It wraps `cargo install` in
# the shared verbose UX: preflight, progress steps, a contextual error trap,
# and disciplined exit codes (0 ok, 1 handled failure, 130 SIGINT, 143 SIGTERM).
# There is no manifest — cargo owns the install record (`.crates2.json` in the
# install root); uninstall.sh wraps `cargo uninstall`.
#
# `cargo install` is the one cargo command that IGNORES the committed
# Cargo.lock by default and re-resolves dependencies. This installer passes
# `--locked` so the installed binary is built from the versions the project
# actually tested. Set INSTALLER_LOCKED=0 to re-resolve instead (useful when a
# pinned dependency no longer builds on a newer rustc).
#
# `--force` keeps the script idempotent and immune to binary-name collisions
# from another crate; a `--path` reinstall of the same package does not
# strictly need it.
#
# Environment:
#   PREFIX              when set, cargo installs into $PREFIX/bin (cargo --root)
#   CARGO_INSTALL_ROOT  honored by cargo when PREFIX is unset
#   INSTALLER_LOCKED=0  drop --locked and let cargo re-resolve dependencies
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
# PROJECT CONFIGURATION — edit this block for your project.
# ============================================================================
project_name="claude-session"
project_root="$_self_dir"
# ============================================================================

installer_locked="${INSTALLER_LOCKED:-1}"

cargo_args=(install --path "$project_root" --force)
if ((installer_locked == 1)); then
  cargo_args+=(--locked)
fi

# Cargo resolves the install root as: --root, then CARGO_INSTALL_ROOT, then the
# `install.root` config value, then CARGO_HOME, then ~/.cargo. Only pin --root
# when the caller asked for one, so a project-level `install.root` config still
# wins; otherwise mirror the remaining chain for reporting only.
if [[ -n ${PREFIX:-} ]]; then
  install_root="$PREFIX"
  install_root_exact=1
  cargo_args+=(--root "$PREFIX")
else
  install_root="${CARGO_INSTALL_ROOT:-${CARGO_HOME:-$HOME/.cargo}}"
  install_root_exact="${CARGO_INSTALL_ROOT:+1}"
  install_root_exact="${install_root_exact:-0}"
fi
bin_dir="$install_root/bin"

trap 'installer_err_trap "$?" "$LINENO" "$BASH_COMMAND"' ERR

installer_set_step "preflight" "A Rust toolchain (cargo) must be installed and on PATH."
installer_step "Preflight"
installer_require_command cargo required "building and installing the crate"
installer_detail "project: $project_name"
installer_detail "crate path: $project_root"
installer_detail "install root: $install_root"
if ((installer_locked == 1)); then
  installer_detail "lockfile: --locked (Cargo.lock is authoritative)"
fi
installer_ok "Preflight"

if ((installer_locked == 1)) && [[ ! -f $project_root/Cargo.lock ]]; then
  installer_warn "no Cargo.lock at $project_root; --locked will fail — commit a lockfile or set INSTALLER_LOCKED=0"
fi

installer_set_step "install crate" \
  "Check the cargo build output above. A --locked failure means Cargo.lock is missing or stale: run 'cargo update', commit the lockfile, or retry with INSTALLER_LOCKED=0."
installer_step "Install crate (cargo install)"
cargo "${cargo_args[@]}"
installer_ok "Install crate"

case ":${PATH}:" in
  *":$bin_dir:"*) ;;
  *) installer_warn "$bin_dir is not on PATH; add it to use the installed binaries" ;;
esac

# shellcheck disable=SC2154 # installer_quiet is assigned in the sourced install-common.sh
if ((installer_quiet != 1)); then
  installer_note "Installed:"
  installer_note "    project: $project_name (via cargo install)"
  if ((install_root_exact == 1)); then
    installer_note "    binaries: $bin_dir (ensure it is on PATH)"
  else
    installer_note "    binaries: $bin_dir (cargo's default root; ensure it is on PATH)"
  fi
  installer_note "    list them: cargo install --list"
fi

printf 'installed %s via cargo\n' "$project_name"
exit 0
