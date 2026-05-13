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

# bats test_tags=unit
@test "sync files does not persist .claude.json by default" {
  mkdir -p "$BATS_TEST_TMPDIR/shared" "$BATS_TEST_TMPDIR/session"
  printf '{"projects":{"/path/a":{"hasTrustDialogAccepted":true}}}\n' \
    >"$BATS_TEST_TMPDIR/session/.claude.json"

  run cs::fn::sync_files out "$BATS_TEST_TMPDIR/session" "$BATS_TEST_TMPDIR/shared"

  assert_success
  [[ ! -e "$BATS_TEST_TMPDIR/shared/.claude.json" ]]
}

# bats test_tags=unit
@test "sync files keeps mtime-gated wholesale copy for non-.claude.json items" {
  mkdir -p "$BATS_TEST_TMPDIR/shared" "$BATS_TEST_TMPDIR/session"
  printf 'shared-newer\n' >"$BATS_TEST_TMPDIR/shared/.credentials.json"
  sleep 1
  printf 'session-older\n' >"$BATS_TEST_TMPDIR/session/.credentials.json"
  touch -d '2 minutes ago' "$BATS_TEST_TMPDIR/session/.credentials.json"

  run cs::fn::sync_files out "$BATS_TEST_TMPDIR/session" "$BATS_TEST_TMPDIR/shared"

  assert_success
  [[ "$(cat "$BATS_TEST_TMPDIR/shared/.credentials.json")" == 'shared-newer' ]]
}

# bats test_tags=unit
@test "sync files cache writeback strips effortLevel, model, and outputStyle" {
  mkdir -p "$BATS_TEST_TMPDIR/shared" "$BATS_TEST_TMPDIR/session"
  printf '{"effortLevel":"high","model":"claude-test","outputStyle":"x","permissions":{"allow":["Bash"]}}\n' \
    >"$BATS_TEST_TMPDIR/session/settings.json"

  run cs::fn::sync_files out "$BATS_TEST_TMPDIR/session" "$BATS_TEST_TMPDIR/shared"

  assert_success
  [[ -f "$XDG_CACHE_HOME/claude-session/settings.json" ]]
  jq -e '
    has("effortLevel") == false
    and has("model") == false
    and has("outputStyle") == false
    and .permissions.allow == ["Bash"]
  ' "$XDG_CACHE_HOME/claude-session/settings.json"
}

# bats test_tags=unit
@test "sync files skips cache writeback when session settings.json is absent" {
  mkdir -p "$BATS_TEST_TMPDIR/shared" "$BATS_TEST_TMPDIR/session"

  run cs::fn::sync_files out "$BATS_TEST_TMPDIR/session" "$BATS_TEST_TMPDIR/shared"

  assert_success
  [[ ! -e "$XDG_CACHE_HOME/claude-session/settings.json" ]]
}

# bats test_tags=unit
@test "sync files cache writeback honors CLAUDE_SESSION_CACHE_DIR and still sanitizes" {
  mkdir -p "$BATS_TEST_TMPDIR/shared" "$BATS_TEST_TMPDIR/session"
  export CLAUDE_SESSION_CACHE_DIR="$BATS_TEST_TMPDIR/custom-cache"
  printf '{"effortLevel":"medium","model":"x","outputStyle":"y","env":{"FOO":"bar"}}\n' \
    >"$BATS_TEST_TMPDIR/session/settings.json"

  run cs::fn::sync_files out "$BATS_TEST_TMPDIR/session" "$BATS_TEST_TMPDIR/shared"

  assert_success
  [[ -f "$BATS_TEST_TMPDIR/custom-cache/settings.json" ]]
  jq -e '
    has("effortLevel") == false
    and has("model") == false
    and has("outputStyle") == false
    and .env.FOO == "bar"
  ' "$BATS_TEST_TMPDIR/custom-cache/settings.json"
}
