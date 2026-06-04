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
@test "sync files out never writes a settings cache" {
  # The composed session settings.json must not be persisted anywhere:
  # versioned layers are the sole compose input, so there is no cross-session
  # (or cross-profile) settings cache to leak through.
  mkdir -p "$BATS_TEST_TMPDIR/shared" "$BATS_TEST_TMPDIR/session"
  printf '{"effortLevel":"high","env":{"CLAUDE_CODE_USE_VERTEX":"1"}}\n' \
    >"$BATS_TEST_TMPDIR/session/settings.json"

  run cs::fn::sync_files out "$BATS_TEST_TMPDIR/session" "$BATS_TEST_TMPDIR/shared"

  assert_success
  [[ ! -e "$XDG_CACHE_HOME/claude-session/settings.json" ]]
  [[ ! -e "$BATS_TEST_TMPDIR/shared/settings.json" ]]
}

# bats test_tags=unit
@test "sync files refuses to round-trip compose artifacts even when overridden" {
  # Regression: CLAUDE_SESSION_SYNC_FILES must not be able to opt the composed
  # settings.json (or its sidecar) into the shared dir. Otherwise profile A's
  # composed settings would be synced out, then synced back in over profile B's
  # fresh composition on the next launch — the exact cross-profile leak this
  # change removes.
  # Include path-decorated aliases that resolve to the same reserved files
  # (./settings.json, foo/../.claude-session-compose.json) to prove the guard
  # is not a naive exact-string match that can be bypassed.
  export CLAUDE_SESSION_SYNC_FILES='settings.json:.claude-session-compose.json:./settings.json:foo/../.claude-session-compose.json'
  mkdir -p "$BATS_TEST_TMPDIR/shared" "$BATS_TEST_TMPDIR/session"
  printf '{"env":{"CLAUDE_CODE_USE_VERTEX":"1"}}\n' \
    >"$BATS_TEST_TMPDIR/session/settings.json"
  printf '{"manifest":"a","layers":[],"env":{}}\n' \
    >"$BATS_TEST_TMPDIR/session/.claude-session-compose.json"

  # out: must not persist either compose artifact to the shared dir.
  run cs::fn::sync_files out "$BATS_TEST_TMPDIR/session" "$BATS_TEST_TMPDIR/shared"
  assert_success
  [[ ! -e "$BATS_TEST_TMPDIR/shared/settings.json" ]]
  [[ ! -e "$BATS_TEST_TMPDIR/shared/.claude-session-compose.json" ]]

  # in: even if a stale artifact somehow exists in the shared dir, it must not
  # be copied over a freshly composed session settings file.
  printf '{"env":{"CLAUDE_CODE_USE_VERTEX":"1"}}\n' \
    >"$BATS_TEST_TMPDIR/shared/settings.json"
  printf '{"env":{"FOO":"bar"}}\n' >"$BATS_TEST_TMPDIR/session/settings.json"
  run cs::fn::sync_files in "$BATS_TEST_TMPDIR/session" "$BATS_TEST_TMPDIR/shared"
  assert_success
  jq -e '.env.FOO == "bar" and (.env | has("CLAUDE_CODE_USE_VERTEX") | not)' \
    "$BATS_TEST_TMPDIR/session/settings.json"
}
