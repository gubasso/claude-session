#!/usr/bin/env bats

setup() {
  load 'test_helper'
  _common_setup
  . "$BATS_TEST_DIRNAME/../lib/helpers.sh"
  cs::helpers::source_fn link_files
}

# bats test_tags=unit
@test "link files creates the home-linked symlink without seeding the canonical file" {
  mkdir -p "$BATS_TEST_TMPDIR/session" "$BATS_TEST_TMPDIR/shared"

  run cs::fn::link_files "$BATS_TEST_TMPDIR/session" "$BATS_TEST_TMPDIR/shared"

  assert_success
  [[ -L "$BATS_TEST_TMPDIR/session/.claude.json" ]]
  [[ "$(readlink "$BATS_TEST_TMPDIR/session/.claude.json")" == "$HOME/.claude.json" ]]
  # Seeding "{}" into $HOME/.claude.json is delegated to the auto-trust
  # critical section so concurrent first-launch terminals do not race;
  # link_files alone must not touch $HOME (also keeps --dry-run side-
  # effect free w.r.t. user-global state).
  [[ ! -e "$HOME/.claude.json" ]]
}

# bats test_tags=unit
@test "link files preserves an existing canonical .claude.json (does not clobber)" {
  mkdir -p "$BATS_TEST_TMPDIR/session" "$BATS_TEST_TMPDIR/shared"
  printf '{"projects":{"/x":{"hasTrustDialogAccepted":true}}}\n' >"$HOME/.claude.json"

  run cs::fn::link_files "$BATS_TEST_TMPDIR/session" "$BATS_TEST_TMPDIR/shared"

  assert_success
  [[ -L "$BATS_TEST_TMPDIR/session/.claude.json" ]]
  jq -e '.projects["/x"].hasTrustDialogAccepted == true' "$HOME/.claude.json"
}

# bats test_tags=unit
@test "link files replaces a stale regular .claude.json in the session dir with the symlink" {
  mkdir -p "$BATS_TEST_TMPDIR/session" "$BATS_TEST_TMPDIR/shared"
  printf '{"stale":true}\n' >"$BATS_TEST_TMPDIR/session/.claude.json"

  run cs::fn::link_files "$BATS_TEST_TMPDIR/session" "$BATS_TEST_TMPDIR/shared"

  assert_success
  [[ -L "$BATS_TEST_TMPDIR/session/.claude.json" ]]
  [[ "$(readlink "$BATS_TEST_TMPDIR/session/.claude.json")" == "$HOME/.claude.json" ]]
}
