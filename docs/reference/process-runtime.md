# Process runtime

Exact contracts for locating, launching, and supervising the child process. For the reasoning, see [the wrapper model](../explanation/wrapper-model.md); for the codes this produces, see [exit codes](./exit-codes.md).

This describes normative design. The crate is pre-implementation.

## Platform scope

Unix is the supported target. The contracts below depend on process groups, POSIX signals, controlling terminals, and Unix file modes. Windows is explicitly out of scope; adding it is a separate decision, not an incremental port, because the signal and process-group model has no direct equivalent.

## Child resolution

The child binary is resolved by an explicit ladder, highest priority first. The first candidate that exists wins — a candidate that exists but fails validation is an error, not a reason to try the next rung. Falling through on a validation failure would silently run a different binary than the user named.

| Priority | Source                                          | Notes                                                                 |
| -------- | ----------------------------------------------- | --------------------------------------------------------------------- |
| 1        | `CLAUDE_SESSION_CHILD_BIN` environment variable | Absolute path. The escape hatch for testing and for unusual installs. |
| 2        | The `child_bin` configuration key               | Absolute path. See [configuration](./configuration.md).               |
| 3        | Search of `PATH` for `claude`                   | The normal case.                                                      |
| 4        | A bundled or vendored path                      | Only if the distribution ships one.                                   |

Each candidate is validated in order:

| Check                                           | Failure              |
| ----------------------------------------------- | -------------------- |
| The path exists                                 | `ChildNotFound`      |
| The path is a regular file or a symlink to one  | `ChildNotFound`      |
| The path is executable by the current user      | `ChildNotExecutable` |
| The path does not resolve to the wrapper itself | `ChildRecursion`     |

The distinction between not-found and not-executable is preserved all the way to the exit code, because the two have completely different fixes.

## Recursion guard

A wrapper installed under the same name as its child, or earlier on `PATH`, will otherwise invoke itself until something breaks. Two independent guards, both required:

**The marker variable.** `CLAUDE_SESSION_REENTRY=1` is set in every child's environment. A wrapper that starts with the marker already set is running as somebody's child; it refuses to resolve a child of its own and fails with `ChildRecursion`.

**The canonical self-check.** The resolved child path is canonicalized — symbolic links followed, `.` and `..` collapsed — and compared against the canonicalized path of the wrapper's own executable. A match is `ChildRecursion`.

Neither guard is sufficient alone. The marker is defeated by an environment scrubbed between the two invocations; the self-check is defeated by a _copy_ of the wrapper rather than a link to it. Together they cover both.

The marker is the one internal variable deliberately left in the child's environment. Every other `CLAUDE_SESSION_*` variable is scrubbed.

## Child environment

The child's environment is built from the parent's, then modified:

| Operation | Keys                                    | Reason                                                                                                          |
| --------- | --------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| Inherit   | Everything else                         | The child is a normal program and needs a normal environment                                                    |
| Remove    | Every `CLAUDE_SESSION_*` key            | The wrapper's internal state is not the child's business, and a nested invocation must not inherit stale values |
| Set       | `CLAUDE_SESSION_REENTRY=1`              | The recursion marker                                                                                            |
| Set       | `CLAUDE_CONFIG_DIR=<session directory>` | The isolation mechanism; see [session isolation](../explanation/session-isolation.md)                           |
| Set       | Composed injections                     | Zero or more keys from configuration or the command line — notably `ANTHROPIC_BASE_URL` for a fronting proxy    |

The composed injections are the **proxy seam**. They are a general mechanism — any key, any value, from configuration or a flag — rather than a proxy-specific feature. `claude-session` implements no proxying, compression, or request rewriting of its own.

Standard input, output, and error are inherited unmodified. The working directory is inherited unmodified.

## Process group topology

**The child shares the wrapper's process group.** The wrapper does not call `setpgid` for the child and does not create a new session.

The consequence, and it is the important one: a terminal-generated signal is delivered by the kernel to **every process in the foreground process group**. Ctrl-C already reaches the child directly. A wrapper that also forwards it delivers the signal twice, and a child that distinguishes one interrupt from two will misread a single keypress.

The alternative topology — giving the child its own process group — means the child no longer receives terminal signals at all, so the wrapper must forward every one _and_ must hand terminal ownership to the child's group so the child can still read input. That is more machinery for a wrapper whose child is always the foreground program.

Shared-group is therefore the choice, and it makes forwarding deliberately **partial**.

## Signal matrix

| Signal               | Terminal broadcasts to the group?    | Wrapper action                                                                                                                                      |
| -------------------- | ------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| `SIGINT` (Ctrl-C)    | Yes                                  | **Do not forward.** The child already received it. The wrapper ignores it for itself and lets the child decide.                                     |
| `SIGQUIT` (Ctrl-\\)  | Yes                                  | **Do not forward.** As above.                                                                                                                       |
| `SIGTSTP` (Ctrl-Z)   | Yes                                  | **Do not forward to the child.** Wait for the child to stop, then re-raise `SIGSTOP` on the wrapper itself so the shell sees the whole job stopped. |
| `SIGCONT`            | Yes                                  | **Do not forward.** Delivered to the group on resume.                                                                                               |
| `SIGWINCH`           | Yes                                  | **Do not forward.** Terminal resize reaches the child directly.                                                                                     |
| `SIGTERM`            | No — usually targeted at one process | **Forward** to the child, then wait.                                                                                                                |
| `SIGHUP`             | Sometimes                            | **Forward** to the child, then wait.                                                                                                                |
| `SIGUSR1`, `SIGUSR2` | No                                   | **Forward** to the child.                                                                                                                           |
| `SIGKILL`, `SIGSTOP` | n/a                                  | Cannot be caught. The child is orphaned and reaped by the init process.                                                                             |

Two rules govern everything in that table:

**Forward what the terminal does not broadcast; stay out of the way for what it does.** "Forward everything" is a bug.

**Re-raise on yourself.** After a forwarded signal kills the child, the wrapper reproduces the child's fate rather than exiting with a translated code — it resets the signal to its default action and re-raises it on itself. This is what makes the wrapper's wait status indistinguishable from the child's to the wrapper's own parent. See [exit codes](./exit-codes.md).

Handlers must be async-signal-safe. The implementation registers a flag in the handler and does the real work on a normal thread; it does not allocate, log, or lock inside a handler.

There is a window between the wrapper starting and the child existing. A signal arriving in that window has no child to reach, and the wrapper emulates the signal's default action on itself rather than swallowing it.

## Spawn and wait

The wrapper spawns and waits; it does not `exec`. The reason is post-flight work — credential and trust sync-back — which never runs if the process image is replaced. See [ADR-0004](../decisions/0004-spawn-and-wait-child-supervision.md).

Sequence:

1. Resolve and validate the child.
2. Build the invocation: binary, argument vector, environment.
3. Prepare the session: directory, credentials, composed settings.
4. Install signal handling.
5. Spawn. Publish the child's process id where the signal machinery can read it.
6. Wait for the child to exit, handling interruption of the wait itself.
7. Clear the published process id.
8. Run post-flight work.
9. Map the child's wait status and exit.

Step 7 before step 8 matters: a signal arriving during post-flight has no child to reach, and forwarding to a dead process id risks hitting an unrelated process that has since reused the number.

The wrapper waits for the specific child it spawned. It does not reap arbitrary children, and it does not install a handler for child-termination signals — there is one child, and its status is collected by waiting.

## Post-flight

Post-flight work runs after the child exits and before the wrapper does:

- Sync project-trust state from the session directory back to the account seed, under a lock, merging conservatively rather than overwriting.
- Update the account's last-used marker.
- Flush logs.

Post-flight failures are **reported but do not change the exit code** of a passthrough invocation. The child's status is the user's answer to "did my command work"; a sync-back failure is a wrapper problem and is reported on standard error. Overwriting a successful child's exit code with a wrapper-internal failure would break every script wrapping this wrapper.

## Further reading

- [Beyond Ctrl-C: the dark corners of Unix signal handling](https://sunshowers.io/posts/beyond-ctrl-c-signals/)
- [Signal handling — Command Line Applications in Rust](https://rust-cli.github.io/book/in-depth/signals.html)
- [`signal-hook`](https://docs.rs/signal-hook/)
- [`rustix`](https://docs.rs/rustix/)
