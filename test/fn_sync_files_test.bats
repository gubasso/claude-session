#!/usr/bin/env bats

setup() {
  load 'test_helper'
  _common_setup
  . "$BATS_TEST_DIRNAME/../lib/helpers.sh"
  cs::helpers::source_fn sync_files
}

# bats test_tags=unit
@test "sync files copies credentials in" {
  mkdir -p "$BATS_TEST_TMPDIR/shared" "$BATS_TEST_TMPDIR/session"
  printf '{}\n' >"$BATS_TEST_TMPDIR/shared/.credentials.json"
  run cs::fn::sync_files in "$BATS_TEST_TMPDIR/session" "$BATS_TEST_TMPDIR/shared"
  assert_success
  [[ -f "$BATS_TEST_TMPDIR/session/.credentials.json" ]]
}
