#!/usr/bin/env bats

setup() {
  load 'test_helper'
  _common_setup
}

# bats test_tags=integration
@test "install and uninstall use manifest in fake home" {
  run env HOME="$HOME" XDG_DATA_HOME="$HOME/.local/share" \
    XDG_STATE_HOME="$XDG_STATE_HOME" XDG_CACHE_HOME="$XDG_CACHE_HOME" \
    "$BATS_TEST_DIRNAME/../install.sh"
  assert_success
  [[ -f "$HOME/.local/bin/claude-session" ]]
  [[ -f "$XDG_STATE_HOME/claude-session/install-manifest" ]]
  run env HOME="$HOME" XDG_DATA_HOME="$HOME/.local/share" \
    XDG_STATE_HOME="$XDG_STATE_HOME" XDG_CACHE_HOME="$XDG_CACHE_HOME" \
    "$BATS_TEST_DIRNAME/../uninstall.sh"
  assert_success
}

# bats test_tags=integration
@test "install scaffolds the runtime-settings cache dir and uninstall removes it" {
  run env HOME="$HOME" XDG_DATA_HOME="$HOME/.local/share" \
    XDG_STATE_HOME="$XDG_STATE_HOME" XDG_CACHE_HOME="$XDG_CACHE_HOME" \
    "$BATS_TEST_DIRNAME/../install.sh"
  assert_success
  [[ -d "$XDG_CACHE_HOME/claude-session" ]]

  # Simulate runtime state inside the cache so we exercise rm -rf, not rmdir.
  printf '{"effortLevel":"medium"}\n' >"$XDG_CACHE_HOME/claude-session/settings.json"

  run env HOME="$HOME" XDG_DATA_HOME="$HOME/.local/share" \
    XDG_STATE_HOME="$XDG_STATE_HOME" XDG_CACHE_HOME="$XDG_CACHE_HOME" \
    "$BATS_TEST_DIRNAME/../uninstall.sh"
  assert_success
  [[ ! -e "$XDG_CACHE_HOME/claude-session" ]]
}
