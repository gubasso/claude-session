#!/usr/bin/env bats
# shellcheck disable=SC2030,SC2031  # bats @test bodies are subshells; env mutations are intentionally local

setup() {
  load 'test_helper'
  _common_setup
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
  assert_output_contains "session_dir="
  [[ ! -f "$BATS_TEST_TMPDIR/child-env" ]]
}

# bats test_tags=integration
@test "run invokes fake claude with args" {
  run claude-session run -- chat hi
  assert_success
  [[ -f "$BATS_TEST_TMPDIR/child-env" ]]
  grep -q 'CLAUDE_CONFIG_DIR=' "$BATS_TEST_TMPDIR/child-env"
  grep -q 'chat hi' "$BATS_TEST_TMPDIR/child-args"
}

# bats test_tags=integration
@test "bare skips failing oauth hook" {
  export CLAUDE_SESSION_OAUTH_CMD='false'
  run claude-session run -- --bare -p ping
  assert_success
}

# bats test_tags=integration
@test "failing oauth hook warns and falls through to native auth" {
  cat >"$BATS_TEST_TMPDIR/fakebin/claude" <<'EOF'
#!/usr/bin/env bash
printf 'CLAUDE_CONFIG_DIR=%s\n' "$CLAUDE_CONFIG_DIR" >"$BATS_TEST_TMPDIR/child-env"
printf 'CLAUDE_CODE_OAUTH_TOKEN=%s\n' "${CLAUDE_CODE_OAUTH_TOKEN-<unset>}" >>"$BATS_TEST_TMPDIR/child-env"
exit 0
EOF
  chmod +x "$BATS_TEST_TMPDIR/fakebin/claude"
  export CLAUDE_SESSION_OAUTH_CMD='false'
  run claude-session run -- chat hi
  assert_success
  assert_output_contains "warning: hook command failed"
  grep -q '^CLAUDE_CODE_OAUTH_TOKEN=<unset>$' "$BATS_TEST_TMPDIR/child-env"
}

# bats test_tags=integration
@test "successful oauth hook exports token to child" {
  cat >"$BATS_TEST_TMPDIR/fakebin/claude" <<'EOF'
#!/usr/bin/env bash
printf 'CLAUDE_CODE_OAUTH_TOKEN=%s\n' "${CLAUDE_CODE_OAUTH_TOKEN-<unset>}" >"$BATS_TEST_TMPDIR/child-env"
exit 0
EOF
  chmod +x "$BATS_TEST_TMPDIR/fakebin/claude"
  export CLAUDE_SESSION_OAUTH_CMD='printf sk-test-token'
  run claude-session run -- chat hi
  assert_success
  grep -q '^CLAUDE_CODE_OAUTH_TOKEN=sk-test-token$' "$BATS_TEST_TMPDIR/child-env"
}
