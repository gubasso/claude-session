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
@test "sync files deep-merges shared and session trust entries in .claude.json" {
  mkdir -p "$BATS_TEST_TMPDIR/shared" "$BATS_TEST_TMPDIR/session"
  cat >"$BATS_TEST_TMPDIR/shared/.claude.json" <<'EOF'
{"projects":{"/path/a":{"hasTrustDialogAccepted":true}}}
EOF
  cat >"$BATS_TEST_TMPDIR/session/.claude.json" <<'EOF'
{"projects":{"/path/b":{"hasTrustDialogAccepted":true}}}
EOF

  run cs::fn::sync_files out "$BATS_TEST_TMPDIR/session" "$BATS_TEST_TMPDIR/shared"

  assert_success
  jq -e '.projects["/path/a"].hasTrustDialogAccepted == true and .projects["/path/b"].hasTrustDialogAccepted == true' \
    "$BATS_TEST_TMPDIR/shared/.claude.json"
}

# bats test_tags=unit
@test "sync files deep-merge preserves session-side keys when shared has different ones" {
  mkdir -p "$BATS_TEST_TMPDIR/shared" "$BATS_TEST_TMPDIR/session"
  cat >"$BATS_TEST_TMPDIR/shared/.claude.json" <<'EOF'
{"projects":{"/path/a":{"hasTrustDialogAccepted":true}},"seenNotifications":{"welcome":true}}
EOF
  cat >"$BATS_TEST_TMPDIR/session/.claude.json" <<'EOF'
{"projects":{"/path/b":{"hasTrustDialogAccepted":true}},"oauthAccount":{"email":"session@example.test"}}
EOF

  run cs::fn::sync_files out "$BATS_TEST_TMPDIR/session" "$BATS_TEST_TMPDIR/shared"

  assert_success
  jq -e '.projects["/path/a"].hasTrustDialogAccepted == true
    and .projects["/path/b"].hasTrustDialogAccepted == true
    and .seenNotifications.welcome == true
    and .oauthAccount.email == "session@example.test"' \
    "$BATS_TEST_TMPDIR/shared/.claude.json"
}

# bats test_tags=unit
@test "sync files warns and keeps shared .claude.json when merge input is malformed" {
  mkdir -p "$BATS_TEST_TMPDIR/shared" "$BATS_TEST_TMPDIR/session"
  printf '{bad json\n' >"$BATS_TEST_TMPDIR/shared/.claude.json"
  printf '{"projects":{"/path/b":{"hasTrustDialogAccepted":true}}}\n' \
    >"$BATS_TEST_TMPDIR/session/.claude.json"

  run cs::fn::sync_files out "$BATS_TEST_TMPDIR/session" "$BATS_TEST_TMPDIR/shared"

  assert_success
  assert_output_contains "warning: .claude.json merge failed; keeping shared copy"
  [[ "$(cat "$BATS_TEST_TMPDIR/shared/.claude.json")" == '{bad json' ]]
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
@test "sync files writes session settings to the default cache path on sync out" {
  mkdir -p "$BATS_TEST_TMPDIR/shared" "$BATS_TEST_TMPDIR/session"
  printf '{"effortLevel":"medium"}\n' >"$BATS_TEST_TMPDIR/session/settings.json"

  run cs::fn::sync_files out "$BATS_TEST_TMPDIR/session" "$BATS_TEST_TMPDIR/shared"

  assert_success
  [[ -f "$XDG_CACHE_HOME/claude-session/settings.json" ]]
  [[ "$(cat "$XDG_CACHE_HOME/claude-session/settings.json")" == "$(cat "$BATS_TEST_TMPDIR/session/settings.json")" ]]
}

# bats test_tags=unit
@test "sync files skips cache writeback when session settings.json is absent" {
  mkdir -p "$BATS_TEST_TMPDIR/shared" "$BATS_TEST_TMPDIR/session"

  run cs::fn::sync_files out "$BATS_TEST_TMPDIR/session" "$BATS_TEST_TMPDIR/shared"

  assert_success
  [[ ! -e "$XDG_CACHE_HOME/claude-session/settings.json" ]]
}

# bats test_tags=unit
@test "sync files respects CLAUDE_SESSION_CACHE_DIR override for cache writeback" {
  mkdir -p "$BATS_TEST_TMPDIR/shared" "$BATS_TEST_TMPDIR/session"
  export CLAUDE_SESSION_CACHE_DIR="$BATS_TEST_TMPDIR/custom-cache"
  printf '{"effortLevel":"medium"}\n' >"$BATS_TEST_TMPDIR/session/settings.json"

  run cs::fn::sync_files out "$BATS_TEST_TMPDIR/session" "$BATS_TEST_TMPDIR/shared"

  assert_success
  [[ -f "$BATS_TEST_TMPDIR/custom-cache/settings.json" ]]
  [[ "$(cat "$BATS_TEST_TMPDIR/custom-cache/settings.json")" == "$(cat "$BATS_TEST_TMPDIR/session/settings.json")" ]]
}
