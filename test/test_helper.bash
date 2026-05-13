#!/usr/bin/env bash
# shellcheck disable=SC2154  # $status and $output are set by bats run

_common_setup() {
  export PATH="${BATS_TEST_DIRNAME}/../bin:$PATH"
  export LIB_DIR="${BATS_TEST_DIRNAME}/../lib"
  export HOME="$BATS_TEST_TMPDIR/home"
  export XDG_CONFIG_HOME="$BATS_TEST_TMPDIR/config"
  export XDG_STATE_HOME="$BATS_TEST_TMPDIR/state"
  export XDG_RUNTIME_DIR="$BATS_TEST_TMPDIR/runtime"
  export XDG_CACHE_HOME="$BATS_TEST_TMPDIR/cache"
  unset CLAUDE_SESSION_CACHE_DIR
  unset CLAUDE_SESSION_CONFIG_DIR
  unset CLAUDE_SESSION_PROFILE
  unset CLAUDE_SESSION_REAL_CLAUDE
  unset CLAUDE_SESSION_OAUTH_CMD
  unset CLAUDE_SESSION_POST_EXIT_CMD
  unset CS_PROFILE_MODE
  unset CS_PROFILE_MANIFEST
  unset CS_COMPOSE_SESSION_DIR
  mkdir -p "$HOME" "$XDG_CONFIG_HOME" "$XDG_STATE_HOME" "$XDG_RUNTIME_DIR" "$XDG_CACHE_HOME"
}

write_manifest() {
  local path=$1
  shift
  mkdir -p "$(dirname "$path")"
  {
    printf 'settings-layers:\n'
    local layer
    for layer in "$@"; do
      printf '  - %s\n' "$layer"
    done
  } >"$path"
}

write_layer() {
  local path=$1
  shift
  mkdir -p "$(dirname "$path")"
  local body='{}'
  local kv=""
  local key=""
  local value=""
  for kv in "$@"; do
    key=${kv%%=*}
    value=${kv#*=}
    body=$(jq --arg key "$key" --arg value "$value" '.env[$key] = $value' <<<"$body")
  done
  printf '%s\n' "$body" >"$path"
}

wipe_profiles() {
  local dir="${CLAUDE_SESSION_CONFIG_DIR:-$XDG_CONFIG_HOME/claude-session}/profiles"
  [[ -d "$dir" ]] || return 0
  find "$dir" -maxdepth 1 -type f -name '*.yaml' -delete
}

skip_if_missing_yq() {
  command -v yq >/dev/null 2>&1 || skip "yq not installed"
  yq --version 2>&1 | grep -q 'mikefarah\|github.com/mikefarah' || skip "yq is not the mikefarah variant"
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
