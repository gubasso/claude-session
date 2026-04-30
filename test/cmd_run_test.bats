#!/usr/bin/env bats
# shellcheck disable=SC2030,SC2031  # bats @test bodies are subshells; env mutations are intentionally local

setup() {
  load 'test_helper'
  _common_setup
  skip_if_missing_yq
  mkdir -p "$BATS_TEST_TMPDIR/fakebin" "$HOME/.claude"
  cat >"$BATS_TEST_TMPDIR/fakebin/claude" <<'EOF'
#!/usr/bin/env bash
printf 'CLAUDE_CONFIG_DIR=%s\n' "$CLAUDE_CONFIG_DIR" >"$BATS_TEST_TMPDIR/child-env"
printf '%s\n' "$*" >"$BATS_TEST_TMPDIR/child-args"
exit "${FAKE_CLAUDE_STATUS:-0}"
EOF
  chmod +x "$BATS_TEST_TMPDIR/fakebin/claude"
  export CLAUDE_SESSION_REAL_CLAUDE="$BATS_TEST_TMPDIR/fakebin/claude"
}

# bats test_tags=integration
@test "run dry-run prints plan without spawning child" {
  run claude-session run --dry-run -- chat hi
  assert_success
  assert_output_contains "mode=stock"
  assert_output_contains "session_dir="
  [[ ! -f "$BATS_TEST_TMPDIR/child-env" ]]
}

# bats test_tags=integration
@test "implicit stock mode writes no settings.json when default.yaml is absent" {
  run claude-session run --dry-run -- chat hi

  assert_success
  assert_output_contains "mode=stock"
  assert_output_contains "settings=not-written"
  local session_dir
  session_dir=$(awk -F= '/^session_dir=/{print $2}' <<<"$output")
  [[ ! -e "$session_dir/settings.json" ]]
}

# bats test_tags=integration
@test "implicit manifest mode composes default.yaml" {
  mkdir -p "$XDG_CONFIG_HOME/claude-session/settings" "$XDG_CONFIG_HOME/claude-session/profiles"
  write_manifest "$XDG_CONFIG_HOME/claude-session/profiles/default.yaml" base
  printf '{"model":"claude-test","env":{"FOO":"bar"}}\n' >"$XDG_CONFIG_HOME/claude-session/settings/base.json"

  run claude-session run --dry-run -- chat hi

  assert_success
  assert_output_contains "mode=manifest"
  assert_output_contains "profile=default"
  local session_dir
  session_dir=$(awk -F= '/^session_dir=/{print $2}' <<<"$output")
  jq -e '.model == "claude-test"' "$session_dir/settings.json"
}

# bats test_tags=integration
@test "explicit profile composes selected manifest" {
  mkdir -p "$XDG_CONFIG_HOME/claude-session/settings" "$XDG_CONFIG_HOME/claude-session/profiles"
  write_manifest "$XDG_CONFIG_HOME/claude-session/profiles/work.yaml" base work
  printf '{"env":{"A":"1"}}\n' >"$XDG_CONFIG_HOME/claude-session/settings/base.json"
  printf '{"env":{"B":"2"}}\n' >"$XDG_CONFIG_HOME/claude-session/settings/work.json"

  run claude-session run --profile work --dry-run -- chat hi

  assert_success
  assert_output_contains "profile=work"
  assert_output_contains "$XDG_CONFIG_HOME/claude-session/settings/base.json:$XDG_CONFIG_HOME/claude-session/settings/work.json"
}

# bats test_tags=integration
@test "run invokes fake claude with args" {
  mkdir -p "$XDG_CONFIG_HOME/claude-session/settings" "$XDG_CONFIG_HOME/claude-session/profiles"
  write_manifest "$XDG_CONFIG_HOME/claude-session/profiles/default.yaml" base
  printf '{"env":{"FOO":"bar"}}\n' >"$XDG_CONFIG_HOME/claude-session/settings/base.json"

  run claude-session run -- chat hi

  assert_success
  [[ -f "$BATS_TEST_TMPDIR/child-env" ]]
  grep -q 'CLAUDE_CONFIG_DIR=' "$BATS_TEST_TMPDIR/child-env"
  grep -q 'chat hi' "$BATS_TEST_TMPDIR/child-args"
}

# bats test_tags=integration
@test "merged env is applied before oauth hook resolution" {
  mkdir -p "$XDG_CONFIG_HOME/claude-session/settings" "$XDG_CONFIG_HOME/claude-session/profiles"
  write_manifest "$XDG_CONFIG_HOME/claude-session/profiles/default.yaml" base
  printf '{"env":{"CLAUDE_SESSION_OAUTH_CMD":"false"}}\n' >"$XDG_CONFIG_HOME/claude-session/settings/base.json"

  run claude-session run --dry-run -- chat hi

  assert_success
  assert_output_contains "oauth_hook=false"
}

# bats test_tags=integration
@test "empty CLAUDE_SESSION_OAUTH_CMD from env block disables oauth" {
  mkdir -p "$XDG_CONFIG_HOME/claude-session/settings" "$XDG_CONFIG_HOME/claude-session/profiles"
  write_manifest "$XDG_CONFIG_HOME/claude-session/profiles/default.yaml" base
  printf '{"env":{"CLAUDE_SESSION_OAUTH_CMD":""}}\n' >"$XDG_CONFIG_HOME/claude-session/settings/base.json"

  run claude-session run --dry-run -- chat hi

  assert_success
  assert_output_contains "oauth_hook=unset"
}
