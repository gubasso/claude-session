# shellcheck shell=bash
: 'desc: Commit a staged home-link file atomically when possible, falling back to in-place rewrite when the destination is a bind-mount target that rejects rename(2) with EBUSY.'

# cs::fn::write_home_link_file <dst> <tmp>
#
# Replace $dst with the contents of $tmp. The function is designed for
# files that live at canonical host paths (typically under $HOME) and
# that the user may bind-mount as single-file mounts into containers
# managed by dctl or similar tools.
#
# Fast path: mv -f "$tmp" "$dst". Atomic rename — readers see either
# the old or new inode, never a half-written file.
#
# Fallback path: when rename fails (typically EBUSY because $dst is a
# mount point in this namespace — see man 2 rename), rewrite $dst's
# existing inode in place. This preserves the bind mount and the
# inode's mode bits (already 0600 by the seed step). $tmp is removed
# either way.
#
# Caller invariants:
#   - Caller holds the appropriate flock for cross-process serialization.
#   - $tmp was created with umask 077 and the desired content already
#     written. (We do not chmod $tmp — the fast path inherits its mode
#     into $dst; the fallback path retains $dst's existing mode.)
#   - $tmp and $dst are on the same filesystem.
#
# Returns 0 on success. Returns non-zero on failure (caller decides the
# user-facing error). On any error, $tmp is removed.
cs::fn::write_home_link_file() {
  local dst=$1
  local tmp=$2

  if mv -f "$tmp" "$dst" 2>/dev/null; then
    return 0
  fi

  cs::helpers::log "rename to $dst rejected (likely a bind-mount target); falling back to in-place rewrite."

  if [[ ! -e "$dst" ]]; then
    (umask 077 && : >"$dst") || { rm -f "$tmp"; return 1; }
    chmod 600 "$dst" || true
  fi

  local new_size
  new_size=$(stat -c '%s' "$tmp" 2>/dev/null) || { rm -f "$tmp"; return 1; }

  if ! dd if="$tmp" of="$dst" bs=64k conv=notrunc,fsync status=none 2>/dev/null; then
    rm -f "$tmp"; return 1
  fi
  if ! truncate -s "$new_size" "$dst" 2>/dev/null; then
    rm -f "$tmp"; return 1
  fi

  rm -f "$tmp"
  return 0
}
