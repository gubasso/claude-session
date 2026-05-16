#!/usr/bin/env bats
# shellcheck disable=SC2030,SC2031  # bats @test bodies are subshells; env mutations are intentionally local

setup() {
  load 'test_helper'
  _common_setup
  skip_if_missing_yq
  mkdir -p "$BATS_TEST_TMPDIR/fakebin" "$HOME/.claude"
  cat >"$BATS_TEST_TMPDIR/fakebin/claude" <<'EOF'
#!/usr/bin/env bash
{
  printf 'CLAUDE_CONFIG_DIR=%s\n' "$CLAUDE_CONFIG_DIR"
  printf 'CLAUDE_CODE_OAUTH_TOKEN=%s\n' "${CLAUDE_CODE_OAUTH_TOKEN-__UNSET__}"
} >"$BATS_TEST_TMPDIR/child-env"
printf '%s\n' "$*" >"$BATS_TEST_TMPDIR/child-args"
exit "${FAKE_CLAUDE_STATUS:-0}"
EOF
  chmod +x "$BATS_TEST_TMPDIR/fakebin/claude"
  export CLAUDE_SESSION_REAL_CLAUDE="$BATS_TEST_TMPDIR/fakebin/claude"
}

__write_run_userns_script() {
  local script=$1
  cat >"$script" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

case "${SCENARIO:-}" in
  auto_trust)
    mkdir -p "$HOME/.claude" "$HOME/project" "$TEST_ROOT"
    src="$TEST_ROOT/src.json"
    dst="$HOME/.claude.json"
    printf '{}\n' >"$src"
    chmod 600 "$src"
    printf '{}\n' >"$dst"
    chmod 600 "$dst"
    mount --bind "$src" "$dst"
    before_inode=$(stat -c '%i' "$dst")
    cd "$HOME/project"
    claude-session run -- chat hi >/dev/null
    after_inode=$(stat -c '%i' "$dst")
    printf 'before_inode=%s\n' "$before_inode"
    printf 'after_inode=%s\n' "$after_inode"
    printf 'mode=%s\n' "$(stat -c '%a' "$dst")"
    jq -e --arg cwd "$PWD" '
      .projects[$cwd].hasTrustDialogAccepted == true
      and .projects[$cwd].hasCompletedProjectOnboarding == true
    ' "$dst" >/dev/null
    ;;
  concurrent)
    mkdir -p "$HOME/.claude" "$HOME/project-a" "$HOME/project-b" "$TEST_ROOT"
    src="$TEST_ROOT/src.json"
    dst="$HOME/.claude.json"
    printf '{}\n' >"$src"
    chmod 600 "$src"
    printf '{}\n' >"$dst"
    chmod 600 "$dst"
    mount --bind "$src" "$dst"
    (
      cd "$HOME/project-a"
      claude-session run -- chat one >/dev/null
    ) &
    pid_a=$!
    (
      cd "$HOME/project-b"
      claude-session run -- chat two >/dev/null
    ) &
    pid_b=$!
    wait "$pid_a"
    wait "$pid_b"
    jq empty "$dst" >/dev/null
    jq -e '
      .projects[env.HOME + "/project-a"].hasTrustDialogAccepted == true
      and .projects[env.HOME + "/project-b"].hasTrustDialogAccepted == true
    ' "$dst" >/dev/null
    ;;
  *)
    printf 'unknown scenario\n' >&2
    exit 1
    ;;
esac
EOF
  chmod +x "$script"
}

# bats test_tags=integration
@test "run dry-run prints plan without spawning child" {
  run claude-session run --dry-run -- chat hi
  assert_success
  assert_output_contains "mode=stock"
  assert_output_contains "session_dir="
  [[ ! -f "$BATS_TEST_TMPDIR/child-env" ]]
}

# bats test_tags=integration
@test "implicit stock mode writes no settings.json when default.yaml is absent" {
  run claude-session run --dry-run -- chat hi

  assert_success
  assert_output_contains "mode=stock"
  assert_output_contains "settings=not-written"
  local session_dir
  session_dir=$(awk -F= '/^session_dir=/{print $2}' <<<"$output")
  [[ ! -e "$session_dir/settings.json" ]]
}

# bats test_tags=integration
@test "implicit manifest mode composes default.yaml" {
  mkdir -p "$XDG_CONFIG_HOME/claude-session/settings" "$XDG_CONFIG_HOME/claude-session/profiles"
  write_manifest "$XDG_CONFIG_HOME/claude-session/profiles/default.yaml" base
  printf '{"model":"claude-test","env":{"FOO":"bar"}}\n' >"$XDG_CONFIG_HOME/claude-session/settings/base.json"

  run claude-session run --dry-run -- chat hi

  assert_success
  assert_output_contains "mode=manifest"
  assert_output_contains "profile=default"
  local session_dir
  session_dir=$(awk -F= '/^session_dir=/{print $2}' <<<"$output")
  jq -e '.model == "claude-test"' "$session_dir/settings.json"
}

# bats test_tags=integration
@test "explicit profile composes selected manifest" {
  mkdir -p "$XDG_CONFIG_HOME/claude-session/settings" "$XDG_CONFIG_HOME/claude-session/profiles"
  write_manifest "$XDG_CONFIG_HOME/claude-session/profiles/work.yaml" base work
  printf '{"env":{"A":"1"}}\n' >"$XDG_CONFIG_HOME/claude-session/settings/base.json"
  printf '{"env":{"B":"2"}}\n' >"$XDG_CONFIG_HOME/claude-session/settings/work.json"

  run claude-session run --profile work --dry-run -- chat hi

  assert_success
  assert_output_contains "profile=work"
  assert_output_contains "$XDG_CONFIG_HOME/claude-session/settings/base.json:$XDG_CONFIG_HOME/claude-session/settings/work.json"
}

# bats test_tags=integration
@test "run invokes fake claude with args" {
  mkdir -p "$XDG_CONFIG_HOME/claude-session/settings" "$XDG_CONFIG_HOME/claude-session/profiles"
  write_manifest "$XDG_CONFIG_HOME/claude-session/profiles/default.yaml" base
  printf '{"env":{"FOO":"bar"}}\n' >"$XDG_CONFIG_HOME/claude-session/settings/base.json"

  run claude-session run -- chat hi

  assert_success
  [[ -f "$BATS_TEST_TMPDIR/child-env" ]]
  grep -q 'CLAUDE_CONFIG_DIR=' "$BATS_TEST_TMPDIR/child-env"
  grep -q 'chat hi' "$BATS_TEST_TMPDIR/child-args"
}

# bats test_tags=integration
@test "run passes CLAUDE_CODE_OAUTH_TOKEN through to the child unchanged" {
  mkdir -p "$XDG_CONFIG_HOME/claude-session/settings" "$XDG_CONFIG_HOME/claude-session/profiles"
  write_manifest "$XDG_CONFIG_HOME/claude-session/profiles/default.yaml" base
  printf '{"env":{"FOO":"bar"}}\n' >"$XDG_CONFIG_HOME/claude-session/settings/base.json"
  export CLAUDE_CODE_OAUTH_TOKEN="sk-ant-oat01-from-parent-shell"

  run claude-session run -- chat hi

  assert_success
  [[ -f "$BATS_TEST_TMPDIR/child-env" ]]
  grep -q '^CLAUDE_CODE_OAUTH_TOKEN=sk-ant-oat01-from-parent-shell$' "$BATS_TEST_TMPDIR/child-env"
}

# bats test_tags=integration
@test "run leaves CLAUDE_CODE_OAUTH_TOKEN unset in the child when unset in the parent" {
  mkdir -p "$XDG_CONFIG_HOME/claude-session/settings" "$XDG_CONFIG_HOME/claude-session/profiles"
  write_manifest "$XDG_CONFIG_HOME/claude-session/profiles/default.yaml" base
  printf '{"env":{"FOO":"bar"}}\n' >"$XDG_CONFIG_HOME/claude-session/settings/base.json"
  unset CLAUDE_CODE_OAUTH_TOKEN

  run claude-session run -- chat hi

  assert_success
  [[ -f "$BATS_TEST_TMPDIR/child-env" ]]
  grep -q '^CLAUDE_CODE_OAUTH_TOKEN=__UNSET__$' "$BATS_TEST_TMPDIR/child-env"
}

# bats test_tags=integration
@test "run auto-trusts the current working directory in the canonical Claude trust file" {
  mkdir -p "$XDG_CONFIG_HOME/claude-session/settings" "$XDG_CONFIG_HOME/claude-session/profiles"
  write_manifest "$XDG_CONFIG_HOME/claude-session/profiles/default.yaml" base
  printf '{"env":{"FOO":"bar"}}\n' >"$XDG_CONFIG_HOME/claude-session/settings/base.json"

  run claude-session run -- chat hi

  assert_success
  jq -e --arg cwd "$PWD" '
    .projects[$cwd].hasTrustDialogAccepted == true
    and .projects[$cwd].hasCompletedProjectOnboarding == true
  ' "$HOME/.claude.json"
}

# bats test_tags=integration
@test "run --dry-run does not mutate the canonical Claude trust file" {
  mkdir -p "$XDG_CONFIG_HOME/claude-session/settings" "$XDG_CONFIG_HOME/claude-session/profiles"
  write_manifest "$XDG_CONFIG_HOME/claude-session/profiles/default.yaml" base
  printf '{"env":{"FOO":"bar"}}\n' >"$XDG_CONFIG_HOME/claude-session/settings/base.json"

  run claude-session run --dry-run -- chat hi

  assert_success
  # Dry-run must not touch $HOME/.claude.json at all: it should not be
  # created when absent, and any pre-existing content must remain
  # unchanged. We assert the stronger "file not created" invariant here
  # because the test starts with a clean $HOME.
  [[ ! -e "$HOME/.claude.json" ]]
}

# bats test_tags=integration
@test "run --dry-run preserves an existing canonical Claude trust file unchanged" {
  mkdir -p "$XDG_CONFIG_HOME/claude-session/settings" "$XDG_CONFIG_HOME/claude-session/profiles"
  write_manifest "$XDG_CONFIG_HOME/claude-session/profiles/default.yaml" base
  printf '{"env":{"FOO":"bar"}}\n' >"$XDG_CONFIG_HOME/claude-session/settings/base.json"
  printf '{"projects":{"/some/other":{"hasTrustDialogAccepted":true}}}\n' >"$HOME/.claude.json"
  chmod 600 "$HOME/.claude.json"
  local before
  before=$(sha256sum "$HOME/.claude.json")

  run claude-session run --dry-run -- chat hi

  assert_success
  local after
  after=$(sha256sum "$HOME/.claude.json")
  [[ "$before" == "$after" ]]
}

# bats test_tags=integration
@test "auto-trust preserves the 0600 mode of an existing canonical trust file" {
  mkdir -p "$XDG_CONFIG_HOME/claude-session/settings" "$XDG_CONFIG_HOME/claude-session/profiles"
  write_manifest "$XDG_CONFIG_HOME/claude-session/profiles/default.yaml" base
  printf '{"env":{"FOO":"bar"}}\n' >"$XDG_CONFIG_HOME/claude-session/settings/base.json"
  printf '{}\n' >"$HOME/.claude.json"
  chmod 600 "$HOME/.claude.json"

  run claude-session run -- chat hi

  assert_success
  local mode
  mode=$(stat -c '%a' "$HOME/.claude.json")
  [[ "$mode" == "600" ]]
}

# bats test_tags=integration
@test "run with CLAUDE_SESSION_AUTO_TRUST_CWD=0 skips auto-trust" {
  mkdir -p "$XDG_CONFIG_HOME/claude-session/settings" "$XDG_CONFIG_HOME/claude-session/profiles"
  write_manifest "$XDG_CONFIG_HOME/claude-session/profiles/default.yaml" base
  printf '{"env":{"FOO":"bar"}}\n' >"$XDG_CONFIG_HOME/claude-session/settings/base.json"
  export CLAUDE_SESSION_AUTO_TRUST_CWD=0

  run claude-session run -- chat hi

  assert_success
  # Even with auto-trust disabled, the home-link target must still be
  # seeded so Claude Code sees a valid file behind the session-side
  # symlink — but no projects[$PWD] entry should be added.
  [[ -f "$HOME/.claude.json" ]]
  jq -e --arg cwd "$PWD" '.projects[$cwd] // null | . == null' "$HOME/.claude.json"
}

# bats test_tags=integration
@test "run seeds every entry in CLAUDE_SESSION_HOME_LINK_FILES" {
  mkdir -p "$XDG_CONFIG_HOME/claude-session/settings" "$XDG_CONFIG_HOME/claude-session/profiles"
  write_manifest "$XDG_CONFIG_HOME/claude-session/profiles/default.yaml" base
  printf '{"env":{"FOO":"bar"}}\n' >"$XDG_CONFIG_HOME/claude-session/settings/base.json"
  export CLAUDE_SESSION_HOME_LINK_FILES=".claude.json:.claude-extra.json"

  run claude-session run -- chat hi

  assert_success
  [[ -f "$HOME/.claude.json" ]]
  [[ -f "$HOME/.claude-extra.json" ]]
  jq -e '. == {}' "$HOME/.claude-extra.json"
  [[ "$(stat -c '%a' "$HOME/.claude-extra.json")" == "600" ]]
}

# bats test_tags=integration
@test "auto-trust prints a notice on first trust and stays silent on re-launch" {
  mkdir -p "$XDG_CONFIG_HOME/claude-session/settings" "$XDG_CONFIG_HOME/claude-session/profiles"
  write_manifest "$XDG_CONFIG_HOME/claude-session/profiles/default.yaml" base
  printf '{"env":{"FOO":"bar"}}\n' >"$XDG_CONFIG_HOME/claude-session/settings/base.json"

  run claude-session run -- chat hi
  assert_success
  [[ "$output" == *"auto-trusted current directory: $PWD"* ]]
  [[ "$output" == *"CLAUDE_SESSION_AUTO_TRUST_CWD=1; set to 0 to disable"* ]]

  run claude-session run -- chat hi
  assert_success
  [[ "$output" != *"auto-trusted current directory"* ]]
}

# bats test_tags=integration
@test "run auto-trusts bind-mounted home trust file in place" {
  skip_if_missing_unshare_rm
  local script="$BATS_TEST_TMPDIR/run-userns.sh"
  __write_run_userns_script "$script"

  run env PATH="$PATH" HOME="$HOME" XDG_CONFIG_HOME="$XDG_CONFIG_HOME" \
    XDG_STATE_HOME="$XDG_STATE_HOME" XDG_RUNTIME_DIR="$XDG_RUNTIME_DIR" \
    XDG_CACHE_HOME="$XDG_CACHE_HOME" \
    CLAUDE_SESSION_REAL_CLAUDE="$CLAUDE_SESSION_REAL_CLAUDE" \
    TEST_ROOT="$BATS_TEST_TMPDIR/run-userns" SCENARIO=auto_trust \
    unshare -rm -- bash "$script"

  assert_success
  local before_inode after_inode
  before_inode=$(awk -F= '/^before_inode=/{print $2}' <<<"$output")
  after_inode=$(awk -F= '/^after_inode=/{print $2}' <<<"$output")
  [[ "$before_inode" == "$after_inode" ]]
  assert_output_contains "mode=600"
}

# bats test_tags=integration
@test "parallel runs serialize bind-mounted auto-trust writes" {
  skip_if_missing_unshare_rm
  local script="$BATS_TEST_TMPDIR/run-userns-concurrent.sh"
  __write_run_userns_script "$script"

  run env PATH="$PATH" HOME="$HOME" XDG_CONFIG_HOME="$XDG_CONFIG_HOME" \
    XDG_STATE_HOME="$XDG_STATE_HOME" XDG_RUNTIME_DIR="$XDG_RUNTIME_DIR" \
    XDG_CACHE_HOME="$XDG_CACHE_HOME" \
    CLAUDE_SESSION_REAL_CLAUDE="$CLAUDE_SESSION_REAL_CLAUDE" \
    TEST_ROOT="$BATS_TEST_TMPDIR/run-userns-concurrent" SCENARIO=concurrent \
    unshare -rm -- bash "$script"

  assert_success
}

# bats test_tags=integration
@test "run dry-run composes versioned effortLevel over stale cache" {
  skip_if_missing_yq
  mkdir -p "$XDG_CONFIG_HOME/claude-session/settings" "$XDG_CONFIG_HOME/claude-session/profiles"
  write_manifest "$XDG_CONFIG_HOME/claude-session/profiles/default.yaml" base
  printf '{"effortLevel":"high"}\n' >"$XDG_CONFIG_HOME/claude-session/settings/base.json"
  mkdir -p "$XDG_CACHE_HOME/claude-session"
  printf '{"effortLevel":"low"}\n' >"$XDG_CACHE_HOME/claude-session/settings.json"

  run claude-session run --dry-run --profile default -- chat hi

  assert_success
  local session_dir
  session_dir=$(awk -F= '/^session_dir=/{print $2}' <<<"$output")
  [[ "$(jq -r '.effortLevel' "$session_dir/settings.json")" == "high" ]]
  jq -e --arg cache "$XDG_CACHE_HOME/claude-session/settings.json" \
    --arg base "$XDG_CONFIG_HOME/claude-session/settings/base.json" '
      .layers[0] == $cache and .layers[1] == $base
    ' "$session_dir/.claude-session-compose.json"
}
