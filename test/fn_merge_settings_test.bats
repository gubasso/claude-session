#!/usr/bin/env bats

setup() {
  load 'test_helper'
  _common_setup
  . "$BATS_TEST_DIRNAME/../lib/helpers.sh"
  cs::helpers::source_fn merge_settings
}

# bats test_tags=unit
@test "merge settings overlays profile json" {
  mkdir -p "$BATS_TEST_TMPDIR/shared" "$BATS_TEST_TMPDIR/session" "$XDG_CONFIG_HOME/claude-session/profiles"
  printf '{"a":1,"b":{"c":1}}\n' >"$BATS_TEST_TMPDIR/shared/settings.base.json"
  printf '{"b":{"d":2}}\n' >"$XDG_CONFIG_HOME/claude-session/profiles/default.settings.json"
  run cs::fn::merge_settings "$BATS_TEST_TMPDIR/shared" default "$BATS_TEST_TMPDIR/session" "$XDG_CONFIG_HOME/claude-session"
  assert_success
  jq -e '.b.d == 2' "$BATS_TEST_TMPDIR/session/settings.json"
}
