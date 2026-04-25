#!/usr/bin/env bash
# shellcheck disable=SC2154  # $status and $output are set by bats run

_common_setup() {
  export PATH="${BATS_TEST_DIRNAME}/../bin:$PATH"
  export LIB_DIR="${BATS_TEST_DIRNAME}/../lib"
  export HOME="$BATS_TEST_TMPDIR/home"
  export XDG_CONFIG_HOME="$BATS_TEST_TMPDIR/config"
  export XDG_STATE_HOME="$BATS_TEST_TMPDIR/state"
  export XDG_RUNTIME_DIR="$BATS_TEST_TMPDIR/runtime"
  unset CLAUDE_SESSION_PROFILE
  unset CLAUDE_SESSION_REAL_CLAUDE
  unset CLAUDE_SESSION_OAUTH_CMD
  unset CLAUDE_SESSION_POST_EXIT_CMD
  mkdir -p "$HOME" "$XDG_CONFIG_HOME" "$XDG_STATE_HOME" "$XDG_RUNTIME_DIR"
}

assert_success() {
  if [[ "$status" -ne 0 ]]; then
    printf 'expected success, got %s\n%s\n' "$status" "$output"
    return 1
  fi
}

assert_failure() {
  if [[ "$status" -eq 0 ]]; then
    printf 'expected failure, got success\n%s\n' "$output"
    return 1
  fi
}

assert_output_contains() {
  local needle=$1
  if [[ "$output" != *"$needle"* ]]; then
    printf 'expected output to contain %s\nactual:\n%s\n' "$needle" "$output"
    return 1
  fi
}
