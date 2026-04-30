#!/usr/bin/env bats

setup() {
  load 'test_helper'
  _common_setup
  skip_if_missing_yq
  mkdir -p "$XDG_CONFIG_HOME/claude-session/profiles" "$XDG_CONFIG_HOME/claude-session/settings"
}

# bats test_tags=unit
@test "profile list reads yaml manifests" {
  write_manifest "$XDG_CONFIG_HOME/claude-session/profiles/default.yaml" base
  write_manifest "$XDG_CONFIG_HOME/claude-session/profiles/work.yaml" base work

  run claude-session profile list

  assert_success
  assert_output_contains "default"
  assert_output_contains "work"
}

# bats test_tags=unit
@test "profile list marks the active profile" {
  write_manifest "$XDG_CONFIG_HOME/claude-session/profiles/default.yaml" base
  write_manifest "$XDG_CONFIG_HOME/claude-session/profiles/work.yaml" base work
  export CLAUDE_SESSION_PROFILE=work

  run claude-session profile list

  assert_success
  assert_output_contains "* work"
}

# bats test_tags=unit
@test "profile list does not mark anything in stock mode" {
  run env -u CLAUDE_SESSION_PROFILE claude-session profile list

  assert_success
  [[ "$output" != *"*"* ]]
}

# bats test_tags=unit
@test "profile show prints manifest layers and redacted env" {
  write_manifest "$XDG_CONFIG_HOME/claude-session/profiles/default.yaml" base
  printf '{"env":{"ANTHROPIC_MODEL":"claude-test","SECRET_KEY":"secret"}}\n' >"$XDG_CONFIG_HOME/claude-session/settings/base.json"

  run claude-session profile show default

  assert_success
  assert_output_contains "# manifest: $XDG_CONFIG_HOME/claude-session/profiles/default.yaml"
  assert_output_contains "#   1: $XDG_CONFIG_HOME/claude-session/settings/base.json"
  assert_output_contains "ANTHROPIC_MODEL=claude-test"
  assert_output_contains "SECRET_KEY=<redacted>"
}

# bats test_tags=unit
@test "profile show --verbose reveals secret-suffixed values" {
  write_manifest "$XDG_CONFIG_HOME/claude-session/profiles/default.yaml" base
  printf '{"env":{"SECRET_KEY":"secret"}}\n' >"$XDG_CONFIG_HOME/claude-session/settings/base.json"

  run claude-session profile show default --verbose

  assert_success
  assert_output_contains "SECRET_KEY=secret"
}

# bats test_tags=unit
@test "profile show on missing profile exits six with three-part error" {
  run claude-session profile show missing

  [[ "$status" -eq 6 ]]
  assert_output_contains "Error: profile \"missing\" not found."
  assert_output_contains "What went wrong:"
  assert_output_contains "How to fix:"
}
