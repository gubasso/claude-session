#!/usr/bin/env bats

setup() {
  load 'test_helper'
  _common_setup
  mkdir -p "$BATS_TEST_TMPDIR/fakebin" "$HOME/.claude"
  printf '#!/usr/bin/env bash\nexit 0\n' >"$BATS_TEST_TMPDIR/fakebin/claude"
  chmod +x "$BATS_TEST_TMPDIR/fakebin/claude"
  export CLAUDE_SESSION_REAL_CLAUDE="$BATS_TEST_TMPDIR/fakebin/claude"
}

# bats test_tags=integration
@test "doctor prints health checks and sessions" {
  run claude-session doctor
  assert_success
  assert_output_contains "config"
  assert_output_contains "Sessions"
}
