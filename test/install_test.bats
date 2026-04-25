#!/usr/bin/env bats

setup() {
  load 'test_helper'
  _common_setup
}

# bats test_tags=integration
@test "install and uninstall use manifest in fake home" {
  run "$BATS_TEST_DIRNAME/../install.sh"
  assert_success
  [[ -f "$HOME/.local/bin/claude-session" ]]
  [[ -f "$XDG_STATE_HOME/claude-session/install-manifest" ]]
  run "$BATS_TEST_DIRNAME/../uninstall.sh"
  assert_success
}
