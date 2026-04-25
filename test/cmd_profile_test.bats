#!/usr/bin/env bats

setup() {
  load 'test_helper'
  _common_setup
  mkdir -p "$XDG_CONFIG_HOME/claude-session/profiles"
}

# bats test_tags=unit
@test "profile list is empty with no profiles" {
  run claude-session profile list
  assert_success
  [[ -z "$output" ]]
}

# bats test_tags=unit
@test "profile show prints profile keys" {
  cat >"$XDG_CONFIG_HOME/claude-session/profiles/default.env" <<'EOF'
ANTHROPIC_MODEL=claude-test
SECRET_KEY=secret
EOF
  run claude-session profile show default
  assert_success
  assert_output_contains "ANTHROPIC_MODEL=claude-test"
  assert_output_contains "SECRET_KEY=<redacted>"
}

# bats test_tags=unit
@test "missing explicit profile exits six" {
  run claude-session --profile missing profile show missing
  [[ "$status" -eq 6 ]]
}
