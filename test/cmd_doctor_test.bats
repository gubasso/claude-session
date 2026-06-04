#!/usr/bin/env bats

setup() {
  load 'test_helper'
  _common_setup
  mkdir -p "$BATS_TEST_TMPDIR/fakebin" "$HOME/.claude"
  printf '#!/usr/bin/env bash\nexit 0\n' >"$BATS_TEST_TMPDIR/fakebin/claude"
  chmod +x "$BATS_TEST_TMPDIR/fakebin/claude"
  export CLAUDE_SESSION_REAL_CLAUDE="$BATS_TEST_TMPDIR/fakebin/claude"
}

# bats test_tags=integration
@test "doctor reports yq availability when manifests exist" {
  skip_if_missing_yq
  mkdir -p "$XDG_CONFIG_HOME/claude-session/settings" "$XDG_CONFIG_HOME/claude-session/profiles"
  write_manifest "$XDG_CONFIG_HOME/claude-session/profiles/default.yaml" base
  printf '{"env":{"FOO":"bar"}}\n' >"$XDG_CONFIG_HOME/claude-session/settings/base.json"
  printf '{}\n' >"$HOME/.claude.json"
  chmod 600 "$HOME/.claude.json"
  : >"$HOME/.claude/.claude-session.lock"

  run claude-session doctor

  assert_success
  assert_output_contains "yq"
  assert_output_contains "mode"
  assert_output_contains "manifest"
  assert_output_contains "home trust"
  assert_output_contains "shared lock"
}

# bats test_tags=integration
@test "doctor reports yq not required in stock mode" {
  run env PATH="${BATS_TEST_DIRNAME}/../bin:$BATS_TEST_TMPDIR/fakebin:/usr/bin:/bin" claude-session doctor

  assert_success
  assert_output_contains "yq"
  assert_output_contains "stock (no manifest)"
}

# Build a PATH that contains the wrapper and symlinks for every external tool
# claude-session uses, EXCEPT yq. Using only this PATH guarantees `command -v yq`
# fails regardless of where yq is installed on the host.
_yqless_path() {
  local sandbox="$BATS_TEST_TMPDIR/yqless-bin"
  mkdir -p "$sandbox"
  local tool
  for tool in jq flock timeout base64 stat sha256sum cksum awk grep mktemp tr cat printf find sleep paste rm rmdir mkdir touch sort uniq head tail wc env bash sed cp mv ln chmod chown date dirname basename readlink id realpath getent tty test true false sh; do
    if command -v "$tool" >/dev/null 2>&1 && [[ ! -e "$sandbox/$tool" ]]; then
      ln -s "$(command -v "$tool")" "$sandbox/$tool"
    fi
  done
  printf '%s:%s:%s' "${BATS_TEST_DIRNAME}/../bin" "$BATS_TEST_TMPDIR/fakebin" "$sandbox"
}

# Sanity-check that the restricted PATH actually hides yq before running doctor,
# so a future host that puts yq somewhere unexpected will fail loudly here.
_assert_yq_hidden() {
  local restricted=$1
  if PATH="$restricted" command -v yq >/dev/null 2>&1; then
    printf 'PATH leak: yq still resolves to %s under restricted PATH\n' \
      "$(PATH="$restricted" command -v yq)"
    return 1
  fi
}

# bats test_tags=integration
@test "doctor warns when yq missing and no manifests" {
  local restricted
  restricted=$(_yqless_path)
  _assert_yq_hidden "$restricted" || return 1

  run env -i HOME="$HOME" XDG_CONFIG_HOME="$XDG_CONFIG_HOME" \
    XDG_STATE_HOME="$XDG_STATE_HOME" XDG_RUNTIME_DIR="$XDG_RUNTIME_DIR" \
    CLAUDE_SESSION_REAL_CLAUDE="$CLAUDE_SESSION_REAL_CLAUDE" \
    LIB_DIR="$LIB_DIR" PATH="$restricted" \
    claude-session doctor

  assert_success
  assert_output_contains "yq"
  assert_output_contains "WARN"
  assert_output_contains "no manifests discovered"
}

# bats test_tags=integration
@test "doctor fails when yq missing but manifests exist" {
  mkdir -p "$XDG_CONFIG_HOME/claude-session/settings" "$XDG_CONFIG_HOME/claude-session/profiles"
  printf 'settings-layers:\n  - base\n' >"$XDG_CONFIG_HOME/claude-session/profiles/default.yaml"
  printf '{"env":{"FOO":"bar"}}\n' >"$XDG_CONFIG_HOME/claude-session/settings/base.json"

  local restricted
  restricted=$(_yqless_path)
  _assert_yq_hidden "$restricted" || return 1

  run env -i HOME="$HOME" XDG_CONFIG_HOME="$XDG_CONFIG_HOME" \
    XDG_STATE_HOME="$XDG_STATE_HOME" XDG_RUNTIME_DIR="$XDG_RUNTIME_DIR" \
    CLAUDE_SESSION_REAL_CLAUDE="$CLAUDE_SESSION_REAL_CLAUDE" \
    LIB_DIR="$LIB_DIR" PATH="$restricted" \
    claude-session doctor

  assert_failure
  assert_output_contains "yq"
  assert_output_contains "FAIL"
  assert_output_contains "Install yq"
}

# bats test_tags=integration
@test "doctor fails on invalid manifest" {
  skip_if_missing_yq
  mkdir -p "$XDG_CONFIG_HOME/claude-session/profiles"
  printf 'settings-layers: foo\n' >"$XDG_CONFIG_HOME/claude-session/profiles/default.yaml"

  run claude-session doctor

  assert_failure
  assert_output_contains "settings-layers must be an array"
}

# bats test_tags=integration
@test "doctor reports stock mode without requiring settings.json" {
  run claude-session doctor

  assert_success
  assert_output_contains "mode"
  assert_output_contains "stock (no manifest)"
  assert_output_contains "Sessions"
  assert_output_contains "home trust"
}

# bats test_tags=integration
@test "doctor --verbose prints config and home trust paths" {
  mkdir -p "$XDG_CONFIG_HOME/claude-session/settings" "$XDG_CONFIG_HOME/claude-session/profiles"
  write_manifest "$XDG_CONFIG_HOME/claude-session/profiles/default.yaml" base
  printf '{"effortLevel":"high"}\n' >"$XDG_CONFIG_HOME/claude-session/settings/base.json"
  printf '{}\n' >"$HOME/.claude.json"

  run claude-session doctor --verbose

  assert_success
  assert_output_contains "CLAUDE_SESSION_CONFIG_DIR="
  assert_output_contains "HOME_TRUST_FILE=$HOME/.claude.json"
}
