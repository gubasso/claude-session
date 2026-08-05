# Process runtime

Exact contracts for locating, launching, and supervising the child process. For the reasoning, see [the wrapper model](../explanation/wrapper-model.md); for the codes this produces, see [exit codes](./exit-codes.md).

This describes normative design. The crate is pre-implementation.

## Platform scope

Linux is the supported target, against child `claude` `2.1.220` or newer ([ADR-0046](../decisions/ADR-0046-support-linux-and-a-single-child-baseline.md)). Every child fact this project records is measured there, and every fixture that captures the child's behaviour is labelled with the version it came from.

Other Unix systems are neither claimed nor deliberately broken. The contracts below depend on process groups, POSIX signals, controlling terminals, and Unix file modes, all of which they have, but nothing is measured on them and no gate proves them. Windows is explicitly out of scope; adding it is a separate decision, not an incremental port, because the signal and process-group model has no direct equivalent.

The baseline is an evidence rule, not a launch gate. The only version check that refuses to spawn is the [child version floor](#child-version-floor) below.

## Child resolution

The child binary is resolved by a ladder of two rungs, highest priority first. A rung whose source is present is terminal: the wrapper validates what that source named and reports the failure, rather than trying the next rung. Falling through on a broken rung would silently run a different binary than the user named ([ADR-0054](../decisions/ADR-0054-resolve-the-child-by-a-terminal-ladder.md)).

| Priority | Source                            | Consulted when                                                |
| -------- | --------------------------------- | ------------------------------------------------------------- |
| 1        | The `child_bin` configuration key | It is set at any layer, `CLAUDE_SESSION_CHILD_BIN` among them |
| 2        | Search of `PATH` for `claude`     | `child_bin` is unset, which is its default. The normal case.  |

`child_bin` is one key, not two rungs. Its layering — environment above project file above user file — belongs to [configuration](./configuration.md#precedence), and an empty environment value is set-to-empty rather than unset, so it reaches validation as a non-absolute path.

Each resolved candidate is validated in order:

| Check                                           | Failure              |
| ----------------------------------------------- | -------------------- |
| A `child_bin` value is an absolute path         | `Config`             |
| The path exists                                 | `ChildNotFound`      |
| The path is a regular file or a symlink to one  | `ChildNotFound`      |
| The path is executable by the current user      | `ChildNotExecutable` |
| The path does not resolve to the wrapper itself | `ChildRecursion`     |

A relative `child_bin` is `Config` (78), not `ChildNotFound`: nothing failed to resolve, the setting is wrong. It is not resolved against the working directory, which Rust's own `Command` documentation calls platform-specific and unstable.

Executability is `access(X_OK)`. That check is advisory — it produces a good diagnostic early and does not guarantee the spawn succeeds; see [spawn and wait](#spawn-and-wait).

The distinction between not-found and not-executable is preserved all the way to the exit code, because the two have completely different fixes.

The resolved path is logged on every invocation, passthrough included, at `info` under `op = resolve_child`, with the absolute path and the rung that produced it. Path resolution is the most common source of surprise in a wrapper installed under several names, and "which binary did it actually run" is the first question of every such report. This costs the child nothing: it is a log record, and the log file's level is independent of the stderr mirror's ([logging and output](./logging-and-output.md)), so the line is present at default verbosity without a byte reaching the terminal.

### Searching `PATH`

The inherited `PATH` is searched left to right for the fixed name `claude`, unmodified — the wrapper does not prune its own directory from it, because a wrapper ahead of its child on `PATH` is a misconfiguration to report as `ChildRecursion`, not to route around.

Zero-length entries are dropped. POSIX calls the zero-length prefix a legacy feature meaning the current working directory, and a bare `::` in `PATH` would mean "run `./claude` from wherever the user happens to be standing" — the wrong thing for a process about to be handed credentials. The `which` crate emulates `which(1)` and does not filter them on Unix, so the wrapper filters them itself.

A candidate rejected for execute permission does not stop the search; it is remembered. If no later entry yields an executable, that memory decides the failure: `ChildNotExecutable` if any candidate was rejected for permission, `ChildNotFound` otherwise. An unset `PATH` is `ChildNotFound`, with no invented default path.

The absolute resolved path is what gets spawned, never the bare name, so the spawn performs no second search of its own and its `errno` names one file.

## Recursion guard

A wrapper installed under the same name as its child, or earlier on `PATH`, will otherwise invoke itself until something breaks. Two independent guards, both required:

The marker variable. `CLAUDE_SESSION_REENTRY=1` is set in every child's environment. A wrapper that starts with the marker already set is running as somebody's child; it refuses to resolve a child of its own and fails with `ChildRecursion`.

The identity self-check. The wrapper's own executable comes from `std::env::current_exe`, which on this target reads `/proc/self/exe`. Both it and the resolved child path are `stat`ed, and equal device and inode is `ChildRecursion` — not equal canonical path strings, which would miss a hard link, since two names for one inode canonicalize to two different paths ([ADR-0055](../decisions/ADR-0055-compare-executable-identity-by-device-and-inode.md)).

Neither guard is sufficient alone. The marker is defeated by an environment scrubbed between the two invocations; the identity check is defeated by a byte-for-byte copy of the wrapper, or by the binary being replaced on disk mid-run. Together they cover both.

A `claude-session` run from inside a Claude Code session refuses to start. The marker reaches the child, and anything the child launches inherits it, so a nested wrapper sees the marker and exits `ChildRecursion` before resolving anything. That is the guard working as specified rather than an edge case to repair: the nested wrapper genuinely cannot tell that invocation apart from the self-invocation loop the marker exists to break.

The marker is the one internal variable deliberately left in the child's environment. Every other `CLAUDE_SESSION_*` key is removed, wrapper inputs included.

## Child environment

The child's environment is a snapshot of the wrapper's own, scrubbed and then added to, in this order. The order is load-bearing: step 3 must follow step 2, or the scrub deletes the marker it just set ([ADR-0057](../decisions/ADR-0057-build-the-child-environment-by-prefix-scrub-and-marker.md)).

1. Snapshot the wrapper's environment, keys and values as OS strings ([coding conventions](./coding-conventions.md#types)).
2. Remove every key whose bytes begin `CLAUDE_SESSION_`, matched ASCII case-sensitively. This sweeps wrapper inputs as well as internals: `CLAUDE_SESSION_CHILD_BIN` and `CLAUDE_SESSION_DEFAULT_PROFILE` are consumed at startup, and a nested wrapper must not re-read a stale one.
3. Set `CLAUDE_SESSION_REENTRY=1`, the recursion marker.
4. Set `CLAUDE_CONFIG_DIR` to the account config directory, selecting the account-wide child state — only when an account is selected.
5. Set `CLAUDE_CODE_OAUTH_TOKEN` to the retrieved token — only in token mode, and only at [step 5 of the spawn sequence](#spawn-and-wait).

Step 2 is the wrapper's only removal, and steps 3 to 5 are its only additions. Everything else the user exported — ambient authentication, `PATH`, locale — reaches the child untouched, which is what makes the child a normal program.

Before launch, the wrapper resolves the stored mode and detects ambient higher-precedence authentication. It may warn on standard error as specified by [accounts](./accounts.md#stored-modes-and-launch-behavior), and never strips ambient authentication.

The proxy seam is inheritance. A user who fronts `claude` with a local proxy exports the child's base-URL variable, and the wrapper hands it over untouched. `claude-session` composes no injections of its own and implements no proxying, compression, or request rewriting; the rejected injection surface is recorded in [ADR-0057](../decisions/ADR-0057-build-the-child-environment-by-prefix-scrub-and-marker.md).

Standard input, output, and error are inherited unmodified. The working directory is inherited unmodified.

## Child argument vector

For a resolved profile, the wrapper constructs one prefix:

```text
--settings <absolute composed/profile-<name>-<digest>.json path>
```

The original child argument vector follows as an untouched suffix. Its order, bytes, count, and `--` sentinel are preserved. The wrapper does not parse, deduplicate, reorder, or reject user tokens. See [ADR-0028](../decisions/ADR-0028-pass-composed-settings-with-the-native-flag.md).

The settings path is carried as an OS string, not a UTF-8 path: it derives from the XDG base directories, whose bytes are arbitrary ([dependencies](./dependencies.md)).

`argv[0]` is the resolved child's absolute path — the default `Command` passes, which the wrapper does not override. It is the one value that cannot misname the file actually running, which is why `ps` and the child agree; the same argument by which [ADR-0055](../decisions/ADR-0055-compare-executable-identity-by-device-and-inode.md) rejected `argv[0]` as an identity signal.

A user-supplied `--settings` replaces the wrapper's composed document. Measured against `claude` 2.1.220 on 2026-07-31, the child keeps only the last occurrence: an earlier settings file is not merged, not validated, and not even read. Since the wrapper's pair is a prefix, the user's own flag always wins and the composed layer is silently discarded. That precedence is accepted rather than repaired — the wrapper cannot detect it without parsing the suffix ([ADR-0047](../decisions/ADR-0047-let-a-user-settings-flag-override-the-group-layer.md)). A user who wants both composes them into one file and passes that.

## Process group topology

The child shares the wrapper's process group. The wrapper does not call `setpgid` for the child and does not create a new session.

The consequence, and it is the important one: a terminal-generated signal is delivered by the kernel to every process in the foreground process group. Ctrl-C already reaches the child directly. A wrapper that also forwards it delivers the signal twice, and a child that distinguishes one interrupt from two will misread a single keypress.

The alternative topology — giving the child its own process group — means the child no longer receives terminal signals at all, so the wrapper must forward every one and must hand terminal ownership to the child's group so the child can still read input. That is more machinery for a wrapper whose child is always the foreground program.

Shared-group is therefore the choice, and it makes forwarding deliberately partial.

## Signal matrix

| Signal               | Terminal broadcasts to the group?    | Wrapper action                                                                                                                                  |
| -------------------- | ------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| `SIGINT` (Ctrl-C)    | Yes                                  | Do not forward. The child already received it. The wrapper ignores it for itself and lets the child decide.                                     |
| `SIGQUIT` (Ctrl-\\)  | Yes                                  | Do not forward. As above.                                                                                                                       |
| `SIGTSTP` (Ctrl-Z)   | Yes                                  | Do not forward to the child. Wait for the child to stop, then re-raise `SIGSTOP` on the wrapper itself so the shell sees the whole job stopped. |
| `SIGCONT`            | Yes                                  | Do not forward. Delivered to the group on resume.                                                                                               |
| `SIGWINCH`           | Yes                                  | Do not forward. Terminal resize reaches the child directly.                                                                                     |
| `SIGTERM`            | No — usually targeted at one process | Forward to the child, then wait.                                                                                                                |
| `SIGHUP`             | Sometimes                            | Forward to the child, then wait.                                                                                                                |
| `SIGUSR1`, `SIGUSR2` | No                                   | Forward to the child.                                                                                                                           |
| `SIGKILL`, `SIGSTOP` | n/a                                  | Cannot be caught. The child is orphaned and reaped by the init process.                                                                         |

Two rules govern everything in that table:

Forward what the terminal does not broadcast; stay out of the way for what it does. "Forward everything" is a bug.

Re-raise on yourself. After a forwarded signal kills the child, the wrapper reproduces the child's fate rather than exiting with a translated code — it resets the signal to its default action and re-raises it on itself. This is what makes the wrapper's wait status indistinguishable from the child's to the wrapper's own parent. See [exit codes](./exit-codes.md).

Handlers must be async-signal-safe. The implementation registers a flag in the handler and does the real work on a normal thread; it does not allocate, log, or lock inside a handler.

There is a window between the wrapper starting and the child existing. A signal arriving in that window has no child to reach, and the wrapper emulates the signal's default action on itself rather than swallowing it.

## Spawn and wait

The wrapper spawns and waits; it does not `exec`. Current supervision and post-flight obligations are recorded in amended [ADR-0004](../decisions/ADR-0004-spawn-and-wait-child-supervision.md).

Sequence:

1. Resolve and validate the child.
2. Resolve the account and stored mode, if selected.
3. Validate private account state and the child-owned config path without reading the child credential.
4. Resolve the profile and ensure its composed settings entry exists.
5. In token mode, retrieve and validate the token immediately before spawn.
6. Build the environment and wrapper-owned argv prefix around the untouched user suffix.
7. Install signal handling.
8. Spawn and publish the child's process id.
9. Wait for the child to exit, then clear the published process id.
10. Run post-flight work.
11. Map the child's wait status and exit.

Step 7 before step 8 matters: a signal arriving during post-flight has no child to reach, and forwarding to a dead process id risks hitting an unrelated process that has since reused the number.

The wrapper waits for the specific child it spawned. It does not reap arbitrary children, and it does not install a handler for child-termination signals — there is one child, and its status is collected by waiting.

Step 8 failing is not the same as the child failing. A spawn that never produced a process leaves the wrapper on its own side of [the boundary](./exit-codes.md#two-regimes), so it exits with a wrapper code — never the child's, since there is no child. Which code depends on what refused, because step 1's checks are advisory and the child can be deleted or `chmod -x`'d in between ([ADR-0056](../decisions/ADR-0056-classify-a-failed-spawn-by-its-cause.md)):

| Spawn failure                      | `err.kind`           | Code |
| ---------------------------------- | -------------------- | ---- |
| `ErrorKind::NotFound` (`ENOENT`)   | `ChildNotFound`      | 127  |
| `ErrorKind::PermissionDenied`      | `ChildNotExecutable` | 126  |
| `raw_os_error() == ENOEXEC`        | `ChildNotExecutable` | 126  |
| Anything else — `ENOMEM`, `EMFILE` | `OsError`            | 71   |

`ENOEXEC` is read through `raw_os_error` because Rust's `io::ErrorKind` has no variant for it; its meaning — found, exec bit set, unloadable — is exactly what 126 names. `OsError` keeps the scope it claims: the machine refused and the wrapper is working correctly. `Internal` (70) still means the wrapper has a bug.

## Post-flight

Post-flight work runs after the child exits and before the wrapper does:

- Update the account's last-used marker.
- Flush logs.

Post-flight failures are reported but do not change the exit code of a passthrough invocation. The child's status is the user's answer to “did my command work”; a marker or log-finalization failure is reported on standard error.

Post-flight deletes nothing, and a wrapper killed before it runs leaves nothing that can fail the next run. What survives a kill, and which run removes it, is in [XDG storage](./xdg-storage.md#cleanup-and-recovery).

## Child version floor

Shared-login correctness depends on child version 2.1.211. Per [ADR-0031](../decisions/ADR-0031-enforce-the-child-refresh-lock-version-floor.md), a `login`-mode launch below that floor fails before spawn, reporting the detected version, the requirement, and the upgrade. An unparsable version fails the same way.

The check is scoped to what depends on the child's refresh lock. `token` mode and a passthrough with no selected account are never blocked by it. The `doctor` probe still reports version state, but it is voluntary and does not stand in for this precondition.

## Further reading

- [Beyond Ctrl-C: the dark corners of Unix signal handling](https://sunshowers.io/posts/beyond-ctrl-c-signals/)
- [Signal handling — Command Line Applications in Rust](https://rust-cli.github.io/book/in-depth/signals.html)
- [`signal-hook`](https://docs.rs/signal-hook/)
- [`rustix`](https://docs.rs/rustix/)
