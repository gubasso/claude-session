#!/usr/bin/env bats

setup() {
  load 'test_helper'
  _common_setup
  skip_if_missing_yq
  . "$BATS_TEST_DIRNAME/../lib/helpers.sh"
  cs::helpers::source_fn compose_profile
  cs::helpers::source_fn apply_profile_env
  export CLAUDE_SESSION_CONFIG_DIR="$XDG_CONFIG_HOME/claude-session"
  mkdir -p "$CLAUDE_SESSION_CONFIG_DIR/settings" "$CLAUDE_SESSION_CONFIG_DIR/profiles" "$BATS_TEST_TMPDIR/session"
}

# bats test_tags=unit
@test "compose_profile writes settings.json from a single layer" {
  write_manifest "$CLAUDE_SESSION_CONFIG_DIR/profiles/default.yaml" base
  printf '{"model":"claude-test","env":{"FOO":"bar"}}\n' >"$CLAUDE_SESSION_CONFIG_DIR/settings/base.json"

  run cs::fn::compose_profile "$CLAUDE_SESSION_CONFIG_DIR/profiles/default.yaml" "$BATS_TEST_TMPDIR/session"

  assert_success
  jq -e '.model == "claude-test"' "$BATS_TEST_TMPDIR/session/settings.json"
}

# bats test_tags=unit
@test "compose_profile applies last-wins merge across layers" {
  write_manifest "$CLAUDE_SESSION_CONFIG_DIR/profiles/default.yaml" base work final
  printf '{"env":{"A":"1","B":"1"}}\n' >"$CLAUDE_SESSION_CONFIG_DIR/settings/base.json"
  printf '{"env":{"B":"2","C":"2"}}\n' >"$CLAUDE_SESSION_CONFIG_DIR/settings/work.json"
  printf '{"env":{"C":"3"}}\n' >"$CLAUDE_SESSION_CONFIG_DIR/settings/final.json"

  run cs::fn::compose_profile "$CLAUDE_SESSION_CONFIG_DIR/profiles/default.yaml" "$BATS_TEST_TMPDIR/session"

  assert_success
  jq -e '.env.A == "1" and .env.B == "2" and .env.C == "3"' "$BATS_TEST_TMPDIR/session/settings.json"
}

# bats test_tags=unit
@test "compose_profile rejects manifest with non-array settings-layers" {
  printf 'settings-layers: foo\n' >"$CLAUDE_SESSION_CONFIG_DIR/profiles/default.yaml"
  printf '{}\n' >"$CLAUDE_SESSION_CONFIG_DIR/settings/base.json"

  run cs::fn::compose_profile "$CLAUDE_SESSION_CONFIG_DIR/profiles/default.yaml" "$BATS_TEST_TMPDIR/session"

  [[ "$status" -eq 3 ]]
  assert_output_contains "settings-layers must be an array"
}

# bats test_tags=unit
@test "compose_profile rejects manifest with empty settings-layers" {
  printf 'settings-layers: []\n' >"$CLAUDE_SESSION_CONFIG_DIR/profiles/default.yaml"

  run cs::fn::compose_profile "$CLAUDE_SESSION_CONFIG_DIR/profiles/default.yaml" "$BATS_TEST_TMPDIR/session"

  [[ "$status" -eq 3 ]]
  assert_output_contains "settings-layers must not be empty"
}

# bats test_tags=unit
@test "compose_profile rejects manifest with extra top-level keys" {
  cat >"$CLAUDE_SESSION_CONFIG_DIR/profiles/default.yaml" <<'EOF'
settings-layers:
  - base
display-name: Default
EOF
  printf '{}\n' >"$CLAUDE_SESSION_CONFIG_DIR/settings/base.json"

  run cs::fn::compose_profile "$CLAUDE_SESSION_CONFIG_DIR/profiles/default.yaml" "$BATS_TEST_TMPDIR/session"

  [[ "$status" -eq 3 ]]
  assert_output_contains "unsupported top-level keys"
}

# bats test_tags=unit
@test "compose_profile rejects missing layer file" {
  write_manifest "$CLAUDE_SESSION_CONFIG_DIR/profiles/default.yaml" base

  run cs::fn::compose_profile "$CLAUDE_SESSION_CONFIG_DIR/profiles/default.yaml" "$BATS_TEST_TMPDIR/session"

  [[ "$status" -eq 3 ]]
  assert_output_contains "settings layer \"base\" not found"
}

# bats test_tags=unit
@test "compose_profile rejects non-string entries in settings-layers" {
  cat >"$CLAUDE_SESSION_CONFIG_DIR/profiles/default.yaml" <<'EOF'
settings-layers:
  - 1
  - true
EOF
  printf '{}\n' >"$CLAUDE_SESSION_CONFIG_DIR/settings/base.json"

  run cs::fn::compose_profile "$CLAUDE_SESSION_CONFIG_DIR/profiles/default.yaml" "$BATS_TEST_TMPDIR/session"

  [[ "$status" -eq 3 ]]
  assert_output_contains "settings-layers entries must be strings"
}

# bats test_tags=unit
@test "compose_profile rejects merged settings root that is not an object" {
  write_manifest "$CLAUDE_SESSION_CONFIG_DIR/profiles/default.yaml" base
  printf '[]\n' >"$CLAUDE_SESSION_CONFIG_DIR/settings/base.json"

  run cs::fn::compose_profile "$CLAUDE_SESSION_CONFIG_DIR/profiles/default.yaml" "$BATS_TEST_TMPDIR/session"

  [[ "$status" -eq 3 ]]
  assert_output_contains "Merged settings root must be a JSON object"
}

# bats test_tags=unit
@test "compose_profile rejects merged env that is not an object" {
  write_manifest "$CLAUDE_SESSION_CONFIG_DIR/profiles/default.yaml" base
  printf '{"env":[]}\n' >"$CLAUDE_SESSION_CONFIG_DIR/settings/base.json"

  run cs::fn::compose_profile "$CLAUDE_SESSION_CONFIG_DIR/profiles/default.yaml" "$BATS_TEST_TMPDIR/session"

  [[ "$status" -eq 3 ]]
  assert_output_contains "settings.env must be an object"
}

# bats test_tags=unit
@test "compose_profile recomposes from current layer content on every call" {
  write_manifest "$CLAUDE_SESSION_CONFIG_DIR/profiles/default.yaml" base
  printf '{"env":{"FOO":"bar"}}\n' >"$CLAUDE_SESSION_CONFIG_DIR/settings/base.json"
  cs::fn::compose_profile "$CLAUDE_SESSION_CONFIG_DIR/profiles/default.yaml" "$BATS_TEST_TMPDIR/session"
  jq -e '.env.FOO == "bar"' "$BATS_TEST_TMPDIR/session/settings.json"

  printf '{"env":{"FOO":"baz"}}\n' >"$CLAUDE_SESSION_CONFIG_DIR/settings/base.json"

  run cs::fn::compose_profile "$CLAUDE_SESSION_CONFIG_DIR/profiles/default.yaml" "$BATS_TEST_TMPDIR/session"

  assert_success
  jq -e '.env.FOO == "baz"' "$BATS_TEST_TMPDIR/session/settings.json"
}

# bats test_tags=unit
@test "compose_profile isolates profiles: one profile's env never leaks into another" {
  # Regression: a Vertex env block defined only in profile A must not appear
  # when composing profile B, even after A has run and synced out.
  write_manifest "$CLAUDE_SESSION_CONFIG_DIR/profiles/vertex.yaml" vertex
  write_manifest "$CLAUDE_SESSION_CONFIG_DIR/profiles/default.yaml" base
  printf '{"env":{"CLAUDE_CODE_USE_VERTEX":"1","ANTHROPIC_VERTEX_PROJECT_ID":"proj"}}\n' \
    >"$CLAUDE_SESSION_CONFIG_DIR/settings/vertex.json"
  printf '{"env":{"FOO":"bar"}}\n' >"$CLAUDE_SESSION_CONFIG_DIR/settings/base.json"

  # Compose + sync-out profile A (vertex), then compose profile B (default).
  cs::fn::compose_profile "$CLAUDE_SESSION_CONFIG_DIR/profiles/vertex.yaml" "$BATS_TEST_TMPDIR/session-a"
  cs::helpers::source_fn sync_files
  cs::fn::sync_files out "$BATS_TEST_TMPDIR/session-a" "$BATS_TEST_TMPDIR/shared"

  run cs::fn::compose_profile "$CLAUDE_SESSION_CONFIG_DIR/profiles/default.yaml" "$BATS_TEST_TMPDIR/session-b"

  assert_success
  jq -e '
    .env.FOO == "bar"
    and (.env | has("CLAUDE_CODE_USE_VERTEX") | not)
    and (.env | has("ANTHROPIC_VERTEX_PROJECT_ID") | not)
  ' "$BATS_TEST_TMPDIR/session-b/settings.json"
}

# bats test_tags=unit
@test "compose_profile overwrites picker-mutated session settings on the next launch" {
  write_manifest "$CLAUDE_SESSION_CONFIG_DIR/profiles/default.yaml" base
  printf '{"effortLevel":"high","model":"claude-test"}\n' \
    >"$CLAUDE_SESSION_CONFIG_DIR/settings/base.json"

  cs::fn::compose_profile \
    "$CLAUDE_SESSION_CONFIG_DIR/profiles/default.yaml" \
    "$BATS_TEST_TMPDIR/session"
  # Simulate Claude Code's /model picker rewriting the session file.
  printf '{"effortLevel":"low","model":"picker-mutated"}\n' \
    >"$BATS_TEST_TMPDIR/session/settings.json"

  # The versioned layers are recomposed unconditionally, so the picker
  # mutation must not survive into the next launch.
  run cs::fn::compose_profile \
    "$CLAUDE_SESSION_CONFIG_DIR/profiles/default.yaml" \
    "$BATS_TEST_TMPDIR/session"

  assert_success
  jq -e '.effortLevel == "high" and .model == "claude-test"' \
    "$BATS_TEST_TMPDIR/session/settings.json"
}

# bats test_tags=unit
@test "compose_profile preserves empty-string env value" {
  write_manifest "$CLAUDE_SESSION_CONFIG_DIR/profiles/default.yaml" base
  printf '{"env":{"CLAUDE_SESSION_POST_EXIT_CMD":""}}\n' >"$CLAUDE_SESSION_CONFIG_DIR/settings/base.json"

  run cs::fn::compose_profile "$CLAUDE_SESSION_CONFIG_DIR/profiles/default.yaml" "$BATS_TEST_TMPDIR/session"

  assert_success
  jq -e '.env.CLAUDE_SESSION_POST_EXIT_CMD == ""' "$BATS_TEST_TMPDIR/session/.claude-session-compose.json"
}

# bats test_tags=unit
@test "apply_profile_env preserves newlines and tabs inside env values" {
  write_manifest "$CLAUDE_SESSION_CONFIG_DIR/profiles/default.yaml" base
  # JSON-encoded value with embedded newline and tab.
  printf '{"env":{"MULTI":"line1\\nline2\\tcol2"}}\n' \
    >"$CLAUDE_SESSION_CONFIG_DIR/settings/base.json"

  cs::fn::compose_profile "$CLAUDE_SESSION_CONFIG_DIR/profiles/default.yaml" "$BATS_TEST_TMPDIR/session"
  cs::fn::apply_profile_env "$BATS_TEST_TMPDIR/session/.claude-session-compose.json"

  local expected
  expected=$(printf 'line1\nline2\tcol2')
  [[ "${MULTI:-}" == "$expected" ]]
}

# bats test_tags=unit
@test "apply_profile_env preserves trailing newline in env values" {
  write_manifest "$CLAUDE_SESSION_CONFIG_DIR/profiles/default.yaml" base
  # Value ending in two newlines — bash command substitution would strip both.
  printf '{"env":{"TAIL":"body\\n\\n"}}\n' \
    >"$CLAUDE_SESSION_CONFIG_DIR/settings/base.json"

  cs::fn::compose_profile "$CLAUDE_SESSION_CONFIG_DIR/profiles/default.yaml" "$BATS_TEST_TMPDIR/session"
  cs::fn::apply_profile_env "$BATS_TEST_TMPDIR/session/.claude-session-compose.json"

  # Use printf+wc to count bytes since [[ ]] equality also works with embedded newlines.
  [[ "${#TAIL}" -eq 6 ]]   # 'body\n\n' is 6 bytes
  [[ "${TAIL: -2}" == $'\n\n' ]]
}

# bats test_tags=unit
@test "apply_profile_env exports merged env in current shell" {
  write_manifest "$CLAUDE_SESSION_CONFIG_DIR/profiles/default.yaml" base
  printf '{"env":{"CLAUDE_SESSION_POST_EXIT_CMD":"","FOO":"bar"}}\n' >"$CLAUDE_SESSION_CONFIG_DIR/settings/base.json"

  cs::fn::compose_profile "$CLAUDE_SESSION_CONFIG_DIR/profiles/default.yaml" "$BATS_TEST_TMPDIR/session"
  cs::fn::apply_profile_env "$BATS_TEST_TMPDIR/session/.claude-session-compose.json"

  [[ "${FOO:-}" == "bar" ]]
  [[ "${CLAUDE_SESSION_POST_EXIT_CMD+x}" == "x" ]]
  [[ "$CLAUDE_SESSION_POST_EXIT_CMD" == "" ]]
}
