# Prior art

Public projects and specifications inspected while designing `claude-session`. They are inspiration only; no code or machine-local resource is a dependency.

Facts marked observed or unverified are externally owned and tracked in [research tracking](./research-tracking.yaml).

Last surveyed: 2026-07-30.

## Authentication and account storage

| Source                                                                                                                             | Pattern                                                   | Lesson for this wrapper                                                             |
| ---------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| [GitHub CLI authentication](https://cli.github.com/manual/gh_auth_login)                                                           | Named login plus token ingestion from standard input      | Account-scoped status and secrets that never enter argv                             |
| [Docker login](https://docs.docker.com/reference/cli/docker/login/)                                                                | `--password-stdin` and external credential-store binaries | Standard input avoids shell history; secure stores belong behind a process boundary |
| [Git credentials](https://git-scm.com/docs/gitcredentials)                                                                         | Helpers exchange credentials out of process               | Prefer an argv helper seam to a linked backend matrix                               |
| [AWS CLI SSO](https://docs.aws.amazon.com/cli/latest/userguide/cli-configure-sso.html)                                             | Account-keyed login cache                                 | Persist one account login rather than copied derived credentials                    |
| [kubectl credential plugins](https://kubernetes.io/docs/reference/access-authn-authz/authentication/#client-go-credential-plugins) | Declared interactive mode                                 | Fail closed when required interaction has no terminal                               |

Claude's rotate-and-revoke behavior and cross-process refresh lock make one shared saved login per account the safe child-specific design. Copying a credential into per-terminal directories evades the child's lock and recreates the concurrent-refresh failure; [ADR-0025](../decisions/0025-share-one-native-login-per-account.md) supersedes that earlier model.

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

| Behavior                                                                                                        | Verification status                                       |
| --------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------- |
| Auth precedence includes ambient cloud/API/helper credentials, then `CLAUDE_CODE_OAUTH_TOKEN`, then saved login | Documented                                                |
| `setup-token` produces a long-lived subscription token for `CLAUDE_CODE_OAUTH_TOKEN`                            | Documented; presentation format deliberately not consumed |
| `CLAUDE_CONFIG_DIR` relocates configuration and saved-login storage on Linux/Windows                            | Documented                                                |
| Ordinary macOS login uses Keychain                                                                              | Documented; per-config-directory namespacing unverified   |
| Processes sharing one saved login coordinate refresh from 2.1.211                                               | Documented and load-bearing                               |
| `--settings` accepts an additional settings document                                                            | Documented; duplicate-flag behavior unverified            |
| `auth status --json` is available for status probing                                                            | Documented; injected-token reporting details tracked      |
| In-TUI `/login` honors relocated config, and its token-mode behavior                                            | Unverified                                                |
| Exact access-token and refresh-grant lifetimes                                                                  | Observed, not guaranteed                                  |

The design floor is child version 2.1.211, enforced at launch by [ADR-0031](../decisions/0031-enforce-the-child-refresh-lock-version-floor.md).

## Baselines

- [Rust CLI Book](https://rust-cli.github.io/book/)
- [Command Line Interface Guidelines](https://clig.dev/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [XDG Base Directory Specification](https://specifications.freedesktop.org/basedir/latest/)
- [Diátaxis](https://diataxis.fr/)
