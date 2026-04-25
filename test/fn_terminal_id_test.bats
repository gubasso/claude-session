#!/usr/bin/env bats

setup() {
  load 'test_helper'
  _common_setup
  . "$BATS_TEST_DIRNAME/../lib/helpers.sh"
  cs::helpers::source_fn terminal_id
}

# bats test_tags=unit
@test "terminal id falls back to pid in bats" {
  run cs::fn::terminal_id
  assert_success
  assert_output_contains "pid-"
}
