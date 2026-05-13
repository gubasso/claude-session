#!/usr/bin/env bats

setup() {
  load 'test_helper'
  _common_setup
  . "$BATS_TEST_DIRNAME/../lib/helpers.sh"
  cs::helpers::source_fn write_home_link_file
}

__write_userns_script() {
  local script=$1
  cat >"$script" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

. "$LIB_DIR/helpers.sh"
cs::helpers::source_fn write_home_link_file

case "${SCENARIO:-}" in
  fallback)
    mkdir -p "$TEST_ROOT"
    src="$TEST_ROOT/src.json"
    dst="$TEST_ROOT/dst.json"
    tmp="$TEST_ROOT/dst.json.tmp"
    printf 'this content is longer than the rewrite\n' >"$src"
    chmod 600 "$src"
    printf 'placeholder\n' >"$dst"
    mount --bind "$src" "$dst"
    before_inode=$(stat -c '%i' "$dst")
    (umask 077 && printf 'short\n' >"$tmp")
    chmod 600 "$tmp"
    if ! cs::fn::write_home_link_file "$dst" "$tmp"; then
      exit 1
    fi
    after_inode=$(stat -c '%i' "$dst")
    printf 'before_inode=%s\n' "$before_inode"
    printf 'after_inode=%s\n' "$after_inode"
    printf 'mode=%s\n' "$(stat -c '%a' "$dst")"
    printf 'size=%s\n' "$(stat -c '%s' "$dst")"
    printf 'content=%s' "$(cat "$dst")"
    [[ ! -e "$tmp" ]]
    ;;
  failure)
    mkdir -p "$TEST_ROOT"
    src="$TEST_ROOT/src.json"
    dst="$TEST_ROOT/dst.json"
    tmp="$TEST_ROOT/dst.json.tmp"
    printf '{}\n' >"$src"
    printf 'placeholder\n' >"$dst"
    mount --bind "$src" "$dst"
    mount -o remount,bind,ro "$dst"
    (umask 077 && printf '{"projects":{}}\n' >"$tmp")
    chmod 600 "$tmp"
    if cs::fn::write_home_link_file "$dst" "$tmp"; then
      printf 'helper unexpectedly succeeded\n' >&2
      exit 1
    fi
    [[ ! -e "$tmp" ]]
    ;;
  *)
    printf 'unknown scenario\n' >&2
    exit 1
    ;;
esac
EOF
  chmod +x "$script"
}

# bats test_tags=unit
@test "write_home_link_file uses atomic rename on regular files" {
  mkdir -p "$BATS_TEST_TMPDIR/home-link"
  local dst="$BATS_TEST_TMPDIR/home-link/.claude.json"
  local tmp="$BATS_TEST_TMPDIR/home-link/.claude.json.tmp"
  printf '{"old":true}\n' >"$dst"
  chmod 600 "$dst"
  local before_inode
  before_inode=$(stat -c '%i' "$dst")
  (umask 077 && printf '{"projects":{"a":1}}\n' >"$tmp")
  chmod 600 "$tmp"

  run cs::fn::write_home_link_file "$dst" "$tmp"

  assert_success
  [[ "$(cat "$dst")" == '{"projects":{"a":1}}' ]]
  [[ "$(stat -c '%a' "$dst")" == "600" ]]
  [[ ! -e "$tmp" ]]
  local after_inode
  after_inode=$(stat -c '%i' "$dst")
  [[ "$after_inode" != "$before_inode" ]]
}

# bats test_tags=integration
@test "write_home_link_file falls back to in-place rewrite on bind-mounted leaf" {
  skip_if_missing_unshare_rm
  local script="$BATS_TEST_TMPDIR/userns-fallback.sh"
  __write_userns_script "$script"

  run env LIB_DIR="$LIB_DIR" TEST_ROOT="$BATS_TEST_TMPDIR/userns-fallback" \
    SCENARIO=fallback \
    unshare -rm -- bash "$script"

  assert_success
  assert_output_contains "rename to $BATS_TEST_TMPDIR/userns-fallback/dst.json rejected (likely a bind-mount target); falling back to in-place rewrite."
  assert_output_contains "before_inode="
  assert_output_contains "after_inode="
  local before_inode after_inode
  before_inode=$(awk -F= '/^before_inode=/{print $2}' <<<"$output")
  after_inode=$(awk -F= '/^after_inode=/{print $2}' <<<"$output")
  [[ "$before_inode" == "$after_inode" ]]
  assert_output_contains "mode=600"
  assert_output_contains "size=6"
  assert_output_contains "content=short"
}

# bats test_tags=integration
@test "write_home_link_file removes staged tmp on fallback failure" {
  skip_if_missing_unshare_rm
  local script="$BATS_TEST_TMPDIR/userns-failure.sh"
  __write_userns_script "$script"

  run env LIB_DIR="$LIB_DIR" TEST_ROOT="$BATS_TEST_TMPDIR/userns-failure" \
    SCENARIO=failure \
    unshare -rm -- bash "$script"

  assert_success
}
