#!/usr/bin/env bats

setup() {
  load 'test_helper'
  _common_setup
}

# bats test_tags=unit
@test "config path prints default config file path" {
  run claude-session config path
  assert_success
  assert_output_contains "$XDG_CONFIG_HOME/claude-session/config.env"
}

# bats test_tags=unit
@test "config show prints defaults" {
  run claude-session config show
  assert_success
  assert_output_contains "CLAUDE_SESSION_PROFILE=default"
  assert_output_contains "CLAUDE_SESSION_SYNC_FILES=.credentials.json"
}

# bats test_tags=integration
@test "config edit creates file with fake editor" {
  EDITOR=true run claude-session config edit
  assert_success
  [[ -f "$XDG_CONFIG_HOME/claude-session/config.env" ]]
}
