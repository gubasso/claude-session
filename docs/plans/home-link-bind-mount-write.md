# Plan: native write strategy for `home-link` files under bind-mount

| Field      | Value                                                       |
|------------|-------------------------------------------------------------|
| Status     | Proposed                                                    |
| Branch     | `15-add-home-link-file-classification-and-auto-trust-cwd-feature` |
| Touches    | `lib/commands/cmd_run.sh`, `lib/functions/fn_write_home_link_file.sh` (new), `lib/commands/cmd_doctor.sh`, `docs/architecture.md`, `docs/config.md`, tests |
| Driver     | Auto-trust write fails with `mv: Device or resource busy` inside dctl containers — the user's default workflow. |

This plan supersedes any prior workaround discussion. The fix lives entirely
inside `claude-session`. No env-var dodge, no per-user opt-out, no
container-side change required to function.

> **For the implementer**: this document is intended to be loaded as a
> self-contained prompt in a fresh session. The Why (§1–§3, §3.1, §8)
> exists so you do not re-litigate the design space; the What (§4–§7) is
> what you should implement. Container-side alternatives have already
> been weighed and rejected — see §3.1. Read §2.1 before considering
> any symlink-based shortcut.

---

## 1. Problem

On startup inside a `dctl`-provisioned dev container, `claude-session`
aborts with:

```
mv: cannot move '/home/<user>/.claude.json.tmp.<pid>' to '/home/<user>/.claude.json': Device or resource busy
```

The container's primary user-facing workflow is multi-container,
multi-session via `devcontainerctl`, with the host `~/.claude.json`
bind-mounted into every container. So the failing path is **the default
case for this user, not an edge case**. Any solution that requires
disabling features, exporting env vars, or changing dctl is by
definition a workaround. We need a native fix in `claude-session`.

### 1.1 Exact failure site

`lib/commands/cmd_run.sh:67-125` — `__run_auto_trust_cwd`:

- `trust_file="$HOME/.claude.json"` (`:75`)
- `(umask 077 && : >"$tmp") … chmod 600 "$tmp"` (`:101-106`) — sets up
  the 0600 invariant on the temp file
- `jq … >"$tmp"` (`:107-113`) — renders updated JSON
- **`mv -f "$tmp" "$trust_file"`** (`:114`) — the failing call
- All under `flock 9>"$lock_file"` (`:84-120`), with
  `lock_file="$shared_dir/.claude-session.lock"` and
  `shared_dir=${CLAUDE_SESSION_SHARED_DIR:-$HOME/.claude}` (`:219`)

The `.tmp.<pid>` suffix in the error message is shell `$$` at `:100`,
which uniquely identifies this code path as the source.

Introduced in commit `e825ed4 feat(session): durable effort + live-shared
trust state`.

## 2. Root cause

Linux `rename(2)` returns `EBUSY` when the destination is a mount point
in the current mount namespace. Quoting `man 2 rename` (ERRORS / EBUSY):

> The rename fails because oldpath or newpath is a directory that is in
> use by some process … or is in use by the system (for example as a
> mount point) …

`dctl` bind-mounts the **single file** `~/.claude.json` (not its parent
directory) into the container — `devcontainers/agents/devcontainer.json`
in `devcontainerctl`:

```json
{ "source": "${localEnv:HOME}/.claude.json",
  "target": "${localEnv:HOME}/.claude.json",
  "type": "bind" }
```

Inside the container that target path is a kernel mount point, so any
caller that uses the canonical "write to a temp file, rename over the
target" atomic-write idiom is rejected by the kernel. This is a
well-known Docker single-file bind-mount trap (see references §9).

The parent directory `~/.claude/` is bind-mounted as a directory in the
same dctl template — replacing files *inside* a bind-mounted directory
is fine; only the **leaf-file mount** triggers `EBUSY`. This is why
other rename-over-tempfile sites in the repo (`fn_sync_files.sh`,
`fn_compose_profile.sh`) are unaffected: they write inside the directory
mount or inside per-pts session dirs that are never mounted.

### 2.1 Kernel fact that closes off the "just symlink it" workaround

`rename(2)` does **not** dereference a symlink at `newpath` — it
**replaces** the symlink with the file at `oldpath`. From `man 2 rename`:

> If `newpath` already exists, it will be atomically replaced … If
> `newpath` refers to a symbolic link, the link will be overwritten.

`mv -f` from coreutils calls `rename(2)`. Consequently, any strategy
that relies on making `$HOME/.claude.json` a symlink (host-side or
in-container) and counting on writes to "follow through" to the target
fails on the **first auto-trust write**: the symlink is replaced with a
regular file at `$HOME/.claude.json`, and live-shared state is lost for
that container. This rules out the otherwise-attractive "make
`~/.claude.json` a symlink into the `~/.claude/` directory mount"
container-side workaround. See §3.1.

## 3. Constraints (from user)

1. Fix lives in `claude-session`. dctl is not touched.
2. Must be native default behavior. No new env var to flip, no opt-out.
3. Must work on the host (regular file, atomic rename available) AND in
   the container (bind-mounted leaf, rename refused).
4. Must preserve the 0600 permission invariant on `$HOME/.claude.json`
   that the current code explicitly establishes (`cmd_run.sh:101-106`,
   originally fixed in Stage 5 finding 2).
5. Must preserve the multi-pts and multi-container concurrency
   guarantees provided by the existing `flock` discipline.
6. No new runtime dependencies beyond what the repo already requires
   (`jq`, util-linux `flock`, coreutils).

### 3.1 Alternatives considered and rejected (container-side / dctl-side)

The user explicitly asked whether the bug can be avoided by changing
how `dctl`/`devcontainer` mounts files, rather than by patching
`claude-session`. The analysis below is durable context for any future
implementer who is tempted to revisit this: the §4 design lives in
`claude-session` because no container-side option preserves all of §3's
constraints simultaneously.

| Alternative | Why rejected |
|---|---|
| **A. Host-side: replace `~/.claude.json` with a symlink into `~/.claude/.claude.json`; drop the single-file bind.** | `rename(2)` overwrites symlinks at `newpath` (§2.1). The first auto-trust write replaces the symlink with a regular file in the container's overlay layer, severing the link to the host's shared file. Live-shared state dies on first write. Would only work if `claude-session` first `realpath`'d the destination — i.e. a code change, which is §4-equivalent. |
| **B. In-container symlink only (created by `postCreateCommand`) pointing $HOME/.claude.json at a path inside the `~/.claude/` directory mount.** | Same failure as A — `rename(2)` clobbers the symlink. Plus adds a postCreate dependency. |
| **C. Use `$CLAUDE_CONFIG_DIR` to relocate `.claude.json` to a shared bind-mounted directory.** | `CLAUDE_CONFIG_DIR` only relocates the `~/.claude/` *directory*, **not** `~/.claude.json`. Tracked upstream at <https://github.com/anthropics/claude-code/issues/33857> (still open). Independently, `claude-session` itself overrides `CLAUDE_CONFIG_DIR` to a per-pts session dir at `cmd_run.sh:248`, so any value the container sets is shadowed inside the wrapper. |
| **D. Mount `$HOME` (or a large slice of it) as a directory instead of mounting `~/.claude.json` as a leaf.** | Blast radius. Overshadows everything else in HOME (shell rc, gpg, ssh, dotfiles, …). Trades a precise bug for a class of new ones. |
| **E. Named Docker volume / tmpfs / overlayfs for the `~/.claude.json` tree.** | Eliminates `EBUSY` because the destination stops being a bind-mount leaf, but **abandons live-shared state** — host edits are no longer visible inside containers and vice versa. This is the dominant pattern in Anthropic's own devcontainer docs (<https://code.claude.com/docs/en/devcontainer>) but is a workflow change, not a fix for the existing workflow. |
| **F. Per-container copy of `~/.claude.json` with a lifecycle sync back to host.** | Same live-shared loss as E, plus reintroduces lost-write races that today's cross-container `flock` discipline (`cmd_run.sh:219`, `fn_sync_files.sh:26`) was built to prevent. Strictly a regression. |
| **G. Mount options: `consistency: cached`/`delegated`, virtiofs, 9p, NFS.** | None of these change the kernel's `EBUSY`-on-mountpoint-rename behavior. Cosmetic. |
| **H. Wait for upstream `claude` to support a config-dir-style override for `.claude.json` (issue #33857).** | Open issue, no ETA. Not a fix the user can deploy. |

**Conclusion**: the only path that satisfies all §3 constraints
together — live-shared across multiple containers and multiple pts,
0600-preserving, lock-inode-preserving across containers, no env-var
opt-out, no upstream `claude` changes — is fixing the write strategy
inside `claude-session`. That is §4 below.

If a future maintainer accepts losing live-shared state for `.claude.json`
(a workflow change, not a bug fix), the named-volume pattern (E) is the
clean dctl-side alternative. That decision is **out of scope** for this
plan.

## 4. Design

### 4.1 Write strategy: try `rename`, fall back to in-place rewrite

The fast path keeps the kernel's atomic-rename guarantee for the
99% of environments where the destination is a regular file. The
fallback path preserves the existing inode (so the bind mount stays
valid) and rewrites its bytes under the existing `flock`.

```
mv -f "$tmp" "$dst"                  # fast path
  └─ on EBUSY → in-place rewrite of "$dst" preserving the inode
```

**Rejected alternative**: always-in-place. It would lose atomic
visibility for readers on the host with no benefit. The bind-mount case
is the default *for this user*, but not for the project.

### 4.2 Detection: no `findmnt` probe

`mv`'s exit status IS the probe. Branch on it. Two reasons:

- `findmnt -T "$dst"` adds a fork+exec to every `claude-session run`.
  Auto-trust is in the hot path.
- The fallback path is cheap when unneeded — we never enter it on a
  successful rename. There is no advantage to predicting the outcome.

A debug log line (`cs::helpers::log`) on the fallback branch is
sufficient for observability.

### 4.3 In-place rewrite mechanism: `dd conv=notrunc,fsync` + `truncate`

| Option | Reject / Accept | Why |
|---|---|---|
| `cat "$tmp" >"$dst"` | Acceptable but weaker | Shell `>` opens with `O_TRUNC` → microsecond window where `$dst` is 0 bytes. Torn-read window includes "empty file". |
| `printf %s "$(<tmp)" >"$dst"` | Reject | Same O_TRUNC issue, plus loads the whole file into shell memory. |
| `python -c "...os.fsync"` | Reject | Adds a runtime dep the project does not require. |
| **`dd if="$tmp" of="$dst" bs=64k conv=notrunc,fsync status=none` then `truncate -s <size> "$dst"`** | **Accept** | Never zero-length (writes from offset 0 in place); `truncate` trims stale tail bytes if the new content is shorter; `conv=fsync` flushes data+metadata; only coreutils. |

The torn-read exposure that remains is the few microseconds between
`dd` start and `truncate` completion — visible only to the real
`claude` binary, which `claude-session` cannot lock against and which
already exposes itself to the same risk by rewriting `~/.claude.json`
in place upstream.

### 4.4 Concurrency across containers — already correct, document the invariant

The lock file is `"$shared_dir/.claude-session.lock"` (`cmd_run.sh:219`,
`:226-227`; `fn_sync_files.sh:26`), where `$shared_dir` defaults to
`$HOME/.claude`. The dctl agents template bind-mounts `~/.claude/` as
a directory, so the lock file resolves to the **same host inode** in
every container.

`flock(LOCK_EX)` on a shared inode serializes across mount and PID
namespaces because Linux locks are VFS-level properties of the inode,
not the namespace view (`man 2 flock`, `man 5 proc_locks`). util-linux
`flock(1)` uses the BSD `flock(2)` syscall — the right primitive for a
whole-file mutex.

**Doctor / docs follow-up** (separate small change, included in this
plan §6.3):

- `cmd_doctor.sh` adds a check that `$shared_dir` resolves to a stable
  host inode across pts namespaces. If a user overrides
  `CLAUDE_SESSION_SHARED_DIR` to a per-container path, the lock loses
  cross-container visibility and concurrent auto-trust writes can
  corrupt `$HOME/.claude.json` regardless of the EBUSY fix. Warn.
- `docs/config.md` documents the invariant under
  `CLAUDE_SESSION_SHARED_DIR`.

### 4.5 Permission invariants

The new code path must preserve the existing 0600 guarantee:

- `(umask 077 && : >"$tmp")` + `chmod 600 "$tmp"` at `cmd_run.sh:105-106`
  stays. The fast path adopts those bits via rename.
- The fallback path writes into an *existing* inode that was seeded
  0600 by `__run_seed_home_link_files` (`cmd_run.sh:60-61`); `dd` does
  not alter mode bits, so 0600 is preserved by definition.
- A trailing `chmod 600 "$trust_file"` after the helper returns serves
  as belt-and-braces and makes the invariant locally obvious.

### 4.6 Scope: generic helper, not a one-off patch

Today only `__run_auto_trust_cwd` (`cmd_run.sh:113-114`) is
EBUSY-sensitive. But `CLAUDE_SESSION_HOME_LINK_FILES` is
user-configurable (`fn_link_files.sh:8`, `cmd_run.sh:46`); any future
home-link target the user adds is a candidate bind-mount leaf. Ship the
helper as a reusable function from day one so future writers don't
re-introduce the bug.

Audit of every `mv …tmp…` site in the repo at the time of writing:

| Site | Target | Mountable leaf? | Action |
|---|---|---|---|
| `lib/commands/cmd_run.sh:113-114` (auto-trust) | `$HOME/.claude.json` | **Yes — the bug** | Switch to helper |
| `lib/commands/cmd_run.sh:21-33` (`__run_write_meta`) | per-pts session dir | No | Unchanged |
| `lib/functions/fn_sync_files.sh:41-46, 54-57` | inside `~/.claude/` dir / `~/.cache/claude-session/` | No (parent is a dir mount, files inside are not mountpoints) | Unchanged |
| `lib/functions/fn_compose_profile.sh:149-162` | per-pts session dir | No | Unchanged |
| `lib/commands/cmd_run.sh:36-65` (`__run_seed_home_link_files`) | `$HOME/<home_link_file>` | Mountable, but writes only when absent (`! -e`) — never replaces | Unchanged |

## 5. Public surface (the helper)

New file: `lib/functions/fn_write_home_link_file.sh`. Matches repo
convention: one public function `cs::fn::<name>`, sourced via
`cs::helpers::source_fn <name>` from `cmd_run.sh`.

```bash
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
```

## 6. Implementation plan

### 6.1 Add the helper

Create `lib/functions/fn_write_home_link_file.sh` with the body from §5.

### 6.2 Wire into the auto-trust path

In `lib/commands/cmd_run.sh`:

1. Source the new function alongside the others (around `:191-199`):

   ```bash
   cs::helpers::source_fn write_home_link_file
   ```

2. Replace the rename block at `:113-119` with a helper call. The full
   replacement for `:107-119`:

   ```bash
       if jq --arg cwd "$cwd" '
         .projects = (.projects // {})
         | .projects[$cwd] = ((.projects[$cwd] // {}) + {
             hasTrustDialogAccepted: true,
             hasCompletedProjectOnboarding: true
           })
       ' "$trust_file" >"$tmp" \
         && cs::fn::write_home_link_file "$trust_file" "$tmp"; then
         chmod 600 "$trust_file" || true   # idempotent belt-and-braces
         : >"$wrote_marker"
       else
         rm -f "$tmp"
         cs::helpers::die 3 "could not persist Claude trust state." "Failed to update $trust_file for $cwd." "  Check file permissions and JSON validity."
       fi
   ```

   Lines `:100-106` (temp staging with umask 077 + chmod 600) are
   **unchanged** — the helper's contract relies on them.

### 6.3 Doctor + config docs

- `lib/commands/cmd_doctor.sh`: add a check after the existing `flock`
  presence check. Resolve `$HOME/.claude.json` via `findmnt -T` (when
  available); report whether it is a bind-mount leaf and which write
  path will fire. Report whether `$shared_dir` resolves to the same
  inode the host sees (or simply whether it lives under `$HOME/.claude`
  by default). This is informational, not a hard failure.
- `docs/config.md`: under `CLAUDE_SESSION_SHARED_DIR`, document that
  the lock at `$shared_dir/.claude-session.lock` must resolve to the
  same host inode across every pts/container that shares
  `~/.claude.json`, otherwise concurrent auto-trust writes are
  unsafe.
- `docs/architecture.md`: extend the "Session-dir lifecycle" /
  home-link section to describe the rename→in-place strategy and
  point at this plan.

## 7. Test plan

The repo has a real test harness (`just check`); these tests must be
added in the same PR.

1. **Unit-ish for the helper** (host case): create a regular file,
   stage a temp, call `cs::fn::write_home_link_file`, assert content
   matches, assert mode is 0600, assert `$tmp` is gone, assert the
   inode number changed (fast path took the rename).

2. **In-place fallback under simulated bind mount**: use
   `unshare -rm` + `mount --bind src tgt` in a temp dir to make a
   single-file mount point that rejects rename, then call the helper
   and assert:
   - Return code 0
   - Content equals `$tmp`'s original content
   - Mode is 0600
   - `$tmp` is gone
   - **Inode number is unchanged** — proving the bind mount survived
   - When the new content is shorter than the old, the file size
     matches the new content (truncate behavior)

3. **`__run_auto_trust_cwd` end-to-end** under the same simulated
   bind mount: run the command, assert
   `.projects[$cwd].hasTrustDialogAccepted == true` in the resulting
   JSON, assert mode 0600, assert inode unchanged.

4. **Concurrency smoke test**: two background `claude-session run`
   invocations with distinct `pts`/`$$` against the same simulated
   bind-mount `~/.claude.json` under a shared lock; assert the
   resulting JSON is valid and contains both project entries (i.e.
   the flock genuinely serialized the in-place rewrites).

5. **Doctor check**: run `claude-session doctor` against the simulated
   bind mount and assert the new line reports "bind-mounted leaf,
   in-place rewrite path active" (or equivalent).

`just check` (pre-commit) must remain green: `shellcheck` and `shfmt`
clean on the new file.

## 8. Out of scope / explicit non-goals

- **Crash-recovery backup file** (`$dst.bak` + invalid-JSON detection
  on next launch). The fallback path is not crash-atomic — a kill
  during `dd` can leave a torn JSON on disk. The same is true of every
  in-place writer of `~/.claude.json` upstream, including the `claude`
  binary itself. If real reports of torn writes appear, revisit; the
  helper is the obvious place to add a backup step.
- **Cross-namespace upstream coordination with the `claude` binary.**
  The torn-read window vs. concurrent `claude` reads is unfixable from
  the wrapper side; only upstream re-architecting `.claude.json` (e.g.
  one file per project under `~/.claude/projects/<hash>.json`) would
  eliminate it.
- **`fsync` of the parent directory** after the in-place rewrite.
  Coreutils gives us `conv=fsync` on the file; there is no portable
  `fsync(dir)` in shell. We accept the same durability ceiling the
  existing temp+rename writers already accept (none of them
  `fsync` the parent dir today).
- **Changes to `dctl`.** Documented as defense-in-depth, not required.
- **Removal or rewrite of the seed step** (`__run_seed_home_link_files`).
  It already handles the "missing file" case under flock; the helper
  only handles "replace existing content".

## 9. References

### Linux / kernel
- `rename(2)` — <https://man7.org/linux/man-pages/man2/rename.2.html>
  (EBUSY on mount-point targets)
- `flock(2)` — <https://man7.org/linux/man-pages/man2/flock.2.html>
- `proc_locks(5)` — <https://man7.org/linux/man-pages/man5/proc_locks.5.html>
  (locks are inode-scoped; PID-namespace only filters visibility)
- LWN — Mount point removal and renaming —
  <https://lwn.net/Articles/570338/>
- LWN — A way to do atomic writes — <https://lwn.net/Articles/789600/>
- GNU coreutils `dd` —
  <https://www.gnu.org/software/coreutils/manual/html_node/dd-invocation.html>
  (`conv=notrunc`, `conv=fsync`)
- File locking in Linux — <https://gavv.net/articles/file-locks/>
- Yakking — Atomic file creation with temporary files —
  <https://yakking.branchable.com/posts/atomic-file-creation-tmpfile/>

### Docker / containers
- Docker bind mounts — <https://docs.docker.com/engine/storage/bind-mounts/>
- Locking files while using bind mounts (Docker forum) —
  <https://forums.docker.com/t/locking-files-while-using-bind-mounts/95350>

### Field reports of the same EBUSY trap
- moby/moby#9295 — `/etc/hostname` Device or resource busy —
  <https://github.com/moby/moby/issues/9295>
- BretFisher/node-docker-good-defaults#28 — EBUSY on rewrite of
  mounted `package.json` —
  <https://github.com/BretFisher/node-docker-good-defaults/issues/28>
- Elasticsearch discuss — single-file `elasticsearch.yml` mount EBUSY —
  <https://discuss.elastic.co/t/when-mounting-elasticsearch-yml-docker-displays-device-or-resource-busy/300981>
- osixia/docker-phpLDAPadmin#15 — sed `-i` over bind mount fails —
  <https://github.com/osixia/docker-phpLDAPadmin/issues/15>

### Repo internals (source-of-truth on conventions and current behavior)
- `lib/helpers.sh:4-12` — `cs::helpers::source_fn` loader pattern
- `lib/commands/cmd_run.sh:67-125` — `__run_auto_trust_cwd` (the
  failing function)
- `lib/commands/cmd_run.sh:36-65` — `__run_seed_home_link_files`
  (companion seed step that creates the 0600 file)
- `lib/commands/cmd_run.sh:191-199` — `source_fn` block where the new
  helper is wired in
- `lib/commands/cmd_run.sh:219` — `shared_dir` resolution
- `lib/functions/fn_link_files.sh:8` —
  `CLAUDE_SESSION_HOME_LINK_FILES` default
- `lib/functions/fn_sync_files.sh:26` — same shared lock file path
- `devcontainerctl/devcontainers/agents/devcontainer.json` (external,
  user-side) — the single-file bind mount that makes this scenario
  the default

### Origin
- `e825ed4 feat(session): durable effort + live-shared trust state` —
  the commit that introduced the failing path.
