# Prior art

Public projects and specifications inspected while designing `claude-session`. They are inspiration only; no code or machine-local resource is a dependency.

Facts marked observed or unverified are externally owned and tracked in [research tracking](./research-tracking.yaml).

Last surveyed: 2026-07-31.

## Argv splitting and flag reservation

How other tools divide a command line between themselves and a program they launch. This is the closest problem class to the wrapper's, and the section [ADR-0043](../decisions/ADR-0043-match-wrapper-flags-by-exact-leading-spelling.md) and [ADR-0044](../decisions/ADR-0044-audit-wrapper-spellings-against-the-child-inventory.md) rest on.

| Source                                                                                                | Pattern                                                                                                           | Taken / rejected                                                                                                                                           |
| ----------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [POSIX Utility Conventions](https://pubs.opengroup.org/onlinepubs/9799919799/basedefs/V1_chap12.html) | Guideline 9, options precede operands; Guideline 10, the first `--` ends options                                  | Both taken; they are the invocation shape and the sentinel                                                                                                 |
| [`gitcli(7)`](https://git-scm.com/docs/gitcli)                                                        | Options before args, the stuck `--opt=value` form preferred, and a warning against relying on prefix abbreviation | Abbreviation ban taken verbatim — a prefix unique today stops being unique when the child ships a flag                                                     |
| [`git` `--end-of-options`](https://git-scm.com/docs/gitcli)                                           | A second terminator, added because `--` had been given a second job                                               | Cautionary: `--` keeps exactly one meaning here                                                                                                            |
| [rustup proxies](https://rust-lang.github.io/rustup/overrides.html)                                   | A `+toolchain` first argument, in a sigil no conforming option can use                                            | Rejected: collision-proof by construction, but it cannot help eight existing verbs or a shipped `--` grammar                                               |
| [`sudo(8)`](https://man7.org/linux/man-pages/man8/sudo.8.html)                                        | `WRAPPER [options] [--] COMMAND [args]`, the canonical launcher shape                                             | Taken; shared with `env`, `timeout`, `xargs`, and `docker run`                                                                                             |
| [kubectl plugins](https://kubernetes.io/docs/tasks/extend-kubectl/kubectl-plugins/)                   | The opposite policy: a built-in command always beats a plugin of the same name                                    | Rejected — the child would have to be enumerated at run time, which is the coupling [ADR-0002](../decisions/ADR-0002-verbatim-argv-passthrough.md) forbids |
| [Commander.js](https://github.com/tj/commander.js)                                                    | The child's own parser: accepts `--opt value`, `--opt=value`, `-o value`, `-ovalue`, bundling, and `--`           | An input, not a model: bundling is why the wrapper never claims a multi-letter short cluster                                                               |
| [`clap`](https://docs.rs/clap/latest/clap/struct.Command.html)                                        | Long-argument inference is opt-in; a trailing variable argument still applies a value delimiter                   | Inference stays off; the trailing-argument route is rejected in favour of the pre-split                                                                    |

The recurring lesson is not to reach for a more permissive parser. It is to reserve a small set of exactly-spelled names, stop deterministically, and treat everything past the stop as opaque.

## Authentication and account storage

| Source                                                                                                                             | Pattern                                                   | Lesson for this wrapper                                                             |
| ---------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| [GitHub CLI authentication](https://cli.github.com/manual/gh_auth_login)                                                           | Named login plus token ingestion from standard input      | Account-scoped status and secrets that never enter argv                             |
| [Docker login](https://docs.docker.com/reference/cli/docker/login/)                                                                | `--password-stdin` and external credential-store binaries | Standard input avoids shell history; secure stores belong behind a process boundary |
| [Git credentials](https://git-scm.com/docs/gitcredentials)                                                                         | Helpers exchange credentials out of process               | Prefer an argv helper seam to a linked backend matrix                               |
| [AWS CLI SSO](https://docs.aws.amazon.com/cli/latest/userguide/cli-configure-sso.html)                                             | Account-keyed login cache                                 | Persist one account login rather than copied derived credentials                    |
| [kubectl credential plugins](https://kubernetes.io/docs/reference/access-authn-authz/authentication/#client-go-credential-plugins) | Declared interactive mode                                 | Fail closed when required interaction has no terminal                               |

Claude's rotate-and-revoke behavior and cross-process refresh lock make one shared saved login per account the safe child-specific design. Copying a credential into per-terminal directories evades the child's lock and recreates the concurrent-refresh failure; [ADR-0025](../decisions/ADR-0025-share-one-native-login-per-account.md) supersedes that earlier model.

## Wrapper and configuration patterns

| Source                                                                       | Pattern                                                            | Taken / rejected                                                         |
| ---------------------------------------------------------------------------- | ------------------------------------------------------------------ | ------------------------------------------------------------------------ |
| [claude-account-switcher](https://github.com/ukogan/claude-account-switcher) | Select a child configuration directory through `CLAUDE_CONFIG_DIR` | Environment selection taken; symlinked mutable credentials rejected      |
| [claude-switch](https://github.com/hoangvu12/claude-switch)                  | Shim resolves and launches the real child                          | Explicit child resolution and recursion guard taken                      |
| [Figment](https://github.com/SergioBenitez/Figment)                          | Layered providers and provenance                                   | Wrapper configuration precedence and per-key provenance taken            |
| [Kustomize](https://github.com/kubernetes-sigs/kustomize)                    | Declared base-and-overlay composition                              | Ordered composition model taken; patch language rejected                 |
| [RFC 7396](https://www.rfc-editor.org/rfc/rfc7396.html)                      | Recursive object merge                                             | Object merge taken; null-delete and universal array replacement rejected |
| [NO_COLOR](https://no-color.org/)                                            | Environment-based color control                                    | Avoids claiming a child flag                                             |

## Filesystem permission checks

How comparable tools validate a private directory they own. The section [ADR-0061](../decisions/ADR-0061-protect-storage-from-accidental-local-drift.md) rests on.

| Source                                                                                                | Mechanism                                                                                                                          | Failure policy                                           | Taken / rejected                                                                                           |
| ----------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| [OpenSSH `safe_path`](https://github.com/openssh/openssh-portable/blob/master/misc.c)                 | `realpath`, then plain `stat` on each ancestor; the source carries an unresolved comment asking whether symlinks should be checked | Refuse, under `StrictModes`                              | Refusal taken; `stat` rejected, since the wrapper's rule is "not a symlink" and needs a non-following call |
| [OpenSSH `safe_path_fd`](https://github.com/openssh/openssh-portable/blob/master/authfile.c)          | Open the target once, then validate that descriptor — "to avoid races". Ancestors stay path-based                                  | Refuse                                                   | Taken verbatim: validate through the handle you already hold                                               |
| [git ownership check](https://git-scm.com/docs/git-config#Documentation/git-config.txt-safedirectory) | One non-following metadata call on three named paths; no component walk, and this is the post-CVE-2022-24765 design                | Refuse, printing the exact command that fixes it         | Refusal shape and the copy-pasteable remedy both taken                                                     |
| [GnuPG homedir check](https://www.gnupg.org/documentation/manuals/gnupg24/gpg.1.html)                 | The file plus one enclosing directory; the source says it stops there deliberately                                                 | **Warn only**, suppressible by `--no-permission-warning` | Rejected: a suppressible warning on a credential path is a footgun, and the flag is a surface with no need |
| [`pass`](https://git.zx2c4.com/password-store/tree/src/password-store.sh)                             | None; `umask 077` at the top of the script                                                                                         | —                                                        | Prevention taken as an addition, never as the whole answer — a umask cannot repair a restored backup       |
| [`age-keygen`](https://github.com/FiloSottile/age/blob/main/cmd/age-keygen/keygen.go)                 | Create exclusively at `0600`, then validate the open handle                                                                        | Warn                                                     | Creation mode taken; the warn-only policy rejected                                                         |
| [systemd `chase`](https://github.com/systemd/systemd/blob/main/src/basic/chase.h)                     | An `openat` walk from the root, whose safe mode refuses any traversal from unprivileged to privileged                              | Refuse                                                   | Rejected: a privilege-boundary model, and PID 1 has a boundary this wrapper does not                       |
| [`tmpfiles.d` `z` and `Z`](https://man.archlinux.org/man/tmpfiles.d.5)                                | Adjust mode and ownership of an existing path, explicitly without following symlinks                                               | Correct in place, silently                               | Correction taken; the silence rejected, since `doctor --strict` is the drift gate                          |
| [`cap-std`](https://github.com/bytecodealliance/cap-std)                                              | Capability directory handles, stated as protection against path traversal and untrusted input                                      | —                                                        | Rejected with the `openat` crate: a dependency whose stated need this project does not have                |

The recurring pattern is narrow: validate the handle you opened, and use ordinary metadata calls on the ancestors. Only the one project with a privilege boundary walks descriptors. OpenSSH's own stated motive is that users accidentally leave a directory world-writable — drift, not an adversary — which is the motive [ADR-0061](../decisions/ADR-0061-protect-storage-from-accidental-local-drift.md) records.

On publishing that scope at all: [restic](https://restic.readthedocs.io/en/stable/100_references.html) and [BorgBackup](https://borgbackup.readthedocs.io/en/stable/internals/security.html) both open with an explicit assumption list — restic's begins by trusting the host the backup is made on, borg's concedes that denial of service is always available to an attacker. [age](https://github.com/C2SP/C2SP/blob/main/age.md) and OpenSSH publish none, and OpenSSH's unresolved source comment is what that costs. A short scope statement, not a formal model, is what a tool this size gets value from.

## Health checks and remediation

| Source                                                                                        | Pattern                                                                                                         | Taken / rejected                                                                                                      |
| --------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------- |
| [`brew doctor`](https://docs.brew.sh/Manpage)                                                 | Named audit methods, listable and individually runnable; one severity tier                                      | Ids and `--list` taken; per-check selection rejected, since `doctor --list` already answers discovery                 |
| [`npm doctor`](https://docs.npmjs.com/cli/v11/commands/npm-doctor)                            | Named check groups as positional selectors                                                                      | Same verdict                                                                                                          |
| [kubeadm preflight](https://kubernetes.io/docs/reference/setup-tools/kubeadm/kubeadm-init.md) | Stable ids, two tiers — warning and blocking error — plus a per-invocation ignore list                          | Two tiers taken as Hard and Soft; the ignore list rejected, since severity is the only waiver lever                   |
| [`fsck(8)`](https://man.archlinux.org/man/fsck.8)                                             | Exit `1` "Filesystem errors corrected" is a different signal from exit `4` "Filesystem errors left uncorrected" | Taken: a corrected mode reports `pass` with detail, never a second check and never a failure                          |
| [clang-tidy](https://clang.llvm.org/extra/clang-tidy/)                                        | A stable check name, its enablement, its promotion to an error, and whether a fix exists are four properties    | Taken as the shape of the catalog row                                                                                 |
| [rustc error index](https://doc.rust-lang.org/error_codes/error-index.html)                   | A stable identifier in the message, the explanation on a page keyed by it                                       | Already the source of the `error[Kind]:` rendering in [exit codes](./exit-codes.md#error-message-shape)               |
| [ShellCheck wiki](https://github.com/koalaman/shellcheck/wiki/SC2086)                         | One page per code, with a fixed internal structure                                                              | Taken in miniature: one remediation per id, in one place                                                              |
| [clig.dev](https://clig.dev/)                                                                 | Catch errors and rewrite them for humans; its worked example puts the fixing command in the message             | Taken; the four-part error shape is this rule made explicit                                                           |
| OpenSSH's "bad ownership or modes" message                                                    | Names the condition precisely, offers no remedy                                                                 | Rejected as a model — it is one of the most-searched error strings there is, which is what the verbatim rule prevents |

## Terminal session identity

How other tools decide "which terminal is this", for the ladder in [ADR-0062](../decisions/ADR-0062-derive-the-group-from-the-controlling-terminal.md) and the claim check in [ADR-0063](../decisions/ADR-0063-claim-a-group-by-its-derivation-fingerprint.md).

| Source                                                                           | Mechanism                                                                                                         | Taken / rejected                                                                                                       |
| -------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| [tmux(1)](https://man.archlinux.org/man/tmux.1)                                  | `pane_tty` and `client_tty` are separate formats: the pane's pseudo-terminal is the server's, the client's is not | The distinction is the whole ladder — rung 2 reads the pane's, which reattaching cannot change                         |
| [screen(1)](https://man7.org/linux/man-pages/man1/screen.1.html)                 | Windows keep running while the session is detached; `$STY` is `pid.tty.host` of the terminal where it was created | Confirms the same split; `$STY` rejected as tool-specific and as session- rather than window-scoped                    |
| [`credentials(7)`](https://man7.org/linux/man-pages/man7/credentials.7.html)     | A terminal is the controlling terminal of at most one session; a session id is the `setsid` caller's process id   | Both taken: the first is why rung 2 cannot collide, the second is rung 3                                               |
| [`proc_pid_stat(5)`](https://man7.org/linux/man-pages/man5/proc_pid_stat.5.html) | Field 22 is start time in clock ticks since boot; field 7 is the controlling terminal's device number             | Start time taken as the discriminator that makes a reused process id a different identity                              |
| [`pam_systemd(8)`](https://man7.org/linux/man-pages/man8/pam_systemd.8.html)     | `XDG_SESSION_ID` is filename-safe and unique per boot                                                             | Rejected on granularity: one login, shared by every pane of it                                                         |
| [atuin](https://github.com/atuinsh/atuin)                                        | Mints a random per-shell UUID in shell initialization                                                             | Rejected as a design, taken as the last rung: without a shell-integration hook a random id is never rediscovered       |
| kitty, WezTerm, iTerm2                                                           | Each exports its own per-window identifier under its own name                                                     | Rejected: mutually incompatible, and inside a multiplexer they describe the client that reattaching replaces           |
| [`script(1)`](https://man7.org/linux/man-pages/man1/script.1.html), asciinema    | Allocate their own pseudo-terminal and run the program under it                                                   | Not a model — they create identity rather than discover it; the consequence is that a run under one gets its own group |
| [Linux devpts](https://www.kernel.org/doc/html/latest/filesystems/devpts.html)   | Pty indices are allocated independently per mount of the filesystem                                               | The source of the container collision ADR-0063 answers                                                                 |
| [`machine-id(5)`](https://man7.org/linux/man-pages/man5/machine-id.5.html)       | Stable per installation, confidential, and absent or empty in an image                                            | Taken as the host discriminator, hashed and with a hostname fallback for exactly those two caveats                     |

Behaviour of externally owned terminals and multiplexers is registered in [research tracking](./research-tracking.yaml) rather than treated as permanent.

## Process supervision

| Source                                                                            | Lesson                                                                                                               |
| --------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| [Beyond Ctrl-C](https://sunshowers.io/posts/beyond-ctrl-c-signals/)               | A shared foreground process group already receives terminal-generated signals; forwarding them again double-delivers |
| [Rust CLI signal handling](https://rust-cli.github.io/book/in-depth/signals.html) | Keep real work outside signal handlers                                                                               |
| [`sysexits(3)`](https://man.freebsd.org/cgi/man.cgi?query=sysexits&sektion=3)     | Wrapper-owned failures use stable categories; child statuses pass through                                            |

## Native child behavior

The owning operational contracts are [accounts](./accounts.md), [configuration](./configuration.md), and [process runtime](./process-runtime.md). Freshness and revalidation procedures live in [research tracking](./research-tracking.yaml).

| Behavior                                                                                                        | Verification status                                            |
| --------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------- |
| Auth precedence includes ambient cloud/API/helper credentials, then `CLAUDE_CODE_OAUTH_TOKEN`, then saved login | Documented                                                     |
| `setup-token` produces a long-lived subscription token for `CLAUDE_CODE_OAUTH_TOKEN`                            | Documented; presentation format deliberately not consumed      |
| `CLAUDE_CONFIG_DIR` relocates configuration and saved-login storage on Linux/Windows                            | Documented                                                     |
| Ordinary macOS login uses Keychain                                                                              | Documented; per-config-directory namespacing unverified        |
| Processes sharing one saved login coordinate refresh from 2.1.211                                               | Documented and load-bearing                                    |
| `--settings` accepts an additional settings document                                                            | Documented                                                     |
| Given repeated `--settings`, only the last is read                                                              | Measured on 2.1.220; earlier files are not merged or validated |
| `--settings` is top-level only and is rejected after a subcommand                                               | Measured on 2.1.220; why the wrapper's pair is a prefix        |
| `--verbose` is a native flag, and `-v` is the native `--version`                                                | Measured on 2.1.220                                            |
| `auth` and `doctor` are native subcommands                                                                      | Measured on 2.1.220                                            |
| `auth status --json` is available for status probing                                                            | Documented; injected-token reporting details tracked           |
| In-TUI `/login` honors relocated config, and its token-mode behavior                                            | Unverified                                                     |
| Exact access-token and refresh-grant lifetimes                                                                  | Observed, not guaranteed                                       |

The design floor is child version 2.1.211, enforced at launch by [ADR-0031](../decisions/ADR-0031-enforce-the-child-refresh-lock-version-floor.md). Everything measured above was measured on Linux against 2.1.220, the documentation baseline set by [ADR-0046](../decisions/ADR-0046-support-linux-and-a-single-child-baseline.md).

## Baselines

- [Rust CLI Book](https://rust-cli.github.io/book/)
- [Command Line Interface Guidelines](https://clig.dev/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [XDG Base Directory Specification](https://specifications.freedesktop.org/basedir/latest/)
- [Diátaxis](https://diataxis.fr/)
