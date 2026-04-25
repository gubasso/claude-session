#!/usr/bin/env bats

setup() {
  load 'test_helper'
  _common_setup
}

# bats test_tags=unit
@test "session list succeeds with no sessions" {
  run claude-session session list
  assert_success
  assert_output_contains "sessions"
}

# bats test_tags=integration
@test "session clean dry-run lists stale directory" {
  mkdir -p "$XDG_RUNTIME_DIR/claude-session/sessions/pid-999999"
  run claude-session session clean --dry-run
  assert_success
  assert_output_contains "pid-999999"
}
