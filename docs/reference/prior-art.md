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
