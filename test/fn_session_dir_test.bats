#!/usr/bin/env bats

setup() {
  load 'test_helper'
  _common_setup
  . "$BATS_TEST_DIRNAME/../lib/helpers.sh"
  cs::helpers::source_fn session_dir
}

# bats test_tags=unit
@test "session dir prefers runtime dir" {
  run cs::fn::session_dir
  assert_success
  assert_output_contains "$XDG_RUNTIME_DIR/claude-session"
}
