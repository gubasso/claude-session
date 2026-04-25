#!/usr/bin/env bats

setup() {
  load 'test_helper'
  _common_setup
}

# bats test_tags=unit
@test "usage exits zero and prints command reference" {
  run claude-session usage
  assert_success
  assert_output_contains "COMMAND TREE"
  assert_output_contains "session clean"
}

# bats test_tags=unit
@test "global help exits zero" {
  run claude-session --help
  assert_success
  assert_output_contains "USAGE"
}

# bats test_tags=unit
@test "version prints version" {
  run claude-session --version
  assert_success
  assert_output_contains "claude-session 0.1.0"
}

# bats test_tags=unit
@test "unknown nested command exits usage error" {
  run claude-session config nope
  assert_failure
  [[ "$status" -eq 2 ]]
}
