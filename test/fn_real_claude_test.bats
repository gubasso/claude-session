#!/usr/bin/env bats

setup() {
  load 'test_helper'
  _common_setup
  . "$BATS_TEST_DIRNAME/../lib/helpers.sh"
  cs::helpers::source_fn real_claude
}

# bats test_tags=unit
@test "real claude uses executable override" {
  printf '#!/usr/bin/env bash\nexit 0\n' >"$BATS_TEST_TMPDIR/claude"
  chmod +x "$BATS_TEST_TMPDIR/claude"
  export CLAUDE_SESSION_REAL_CLAUDE="$BATS_TEST_TMPDIR/claude"
  run cs::fn::real_claude
  assert_success
  assert_output_contains "$BATS_TEST_TMPDIR/claude"
}
