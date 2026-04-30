# Install

`claude-session` installs as a multi-file layout: a shim in `bin/` plus
a `lib/` tree and a completions script. The `justfile` orchestrates
install, uninstall, test, and lint.

## Install matrix

XDG-aware, `PREFIX`-overridable. The **user** column is the default
(no `PREFIX`); the **system** column kicks in when `PREFIX` is set
(e.g. `PREFIX=/usr/local`).

| Artifact          | User                                                   | System                                                    |
|-------------------|--------------------------------------------------------|-----------------------------------------------------------|
| binary            | `$HOME/.local/bin/claude-session`                      | `$PREFIX/bin/claude-session`                              |
| lib tree          | `$HOME/.local/lib/claude-session/`                     | `$PREFIX/lib/claude-session/`                             |
| bash completion   | `$XDG_DATA_HOME/bash-completion/completions/`          | `$(pkg-config --variable=completionsdir bash-completion)` |
| config            | `$XDG_CONFIG_HOME/claude-session/config.env`           | `/etc/claude-session/config.env`                          |
| state             | `${XDG_STATE_HOME:-$HOME/.local/state}/claude-session/` | `/var/lib/claude-session/`                                |

- User-install detection: `[[ $EUID -ne 0 && -z "$PREFIX" ]]`.
- `install.sh` writes a manifest at
  `${XDG_STATE_HOME:-$HOME/.local/state}/claude-session/install-manifest`
  (or `/var/lib/claude-session/install-manifest` for system installs)
  containing every path it copied. `uninstall.sh` reads this manifest
  and removes exactly those paths; it never deletes user config or
  session state.
- The state dir doubles as the XDG-spec session-root fallback when
  `XDG_RUNTIME_DIR` is unset. Sessions live in a `sessions/` subdir
  there and are ignored by uninstall (see "Manifest-based uninstall"
  below).

## Prerequisites

- bash 4.4+ (for `inherit_errexit`).
- `jq` — settings-overlay merge at runtime.
- `yq` (mikefarah v4+) — profile manifest parsing at runtime.
- `base64` — lossless transport of merged-env values into shell variables
  (preserves embedded tabs, newlines, and trailing newlines). Standard
  in GNU coreutils; macOS / BSD ship a compatible `base64 -d`.
- `flock` — exit-time sync safety under concurrent terminals.
- `just` — recipe orchestration. Optional for install (`install.sh`
  works standalone), required for the test/lint workflow.

## Recipes

The `justfile` is the only supported entry point for
build/test/lint/install operations. Its recipes:

| Recipe                                  | Purpose                                                                                                                    |
|-----------------------------------------|----------------------------------------------------------------------------------------------------------------------------|
| `just` (no args; default)               | `@just --list` — print recipe names with their doc-comments. Matches the convention used across the repo owner's other CLIs. |
| `just install`                          | Run `install.sh` for user scope (`$HOME/.local/...`). Writes install manifest.                                             |
| `just install PREFIX=/usr/local`        | System install under `$PREFIX`. Usually run with `sudo`.                                                                  |
| `just uninstall`                        | Read the install manifest, remove every path it lists. Warn if the manifest is missing.                                   |
| `just test`                             | Run unit **and** integration tests (`just test-unit && just test-integration`).                                            |
| `just test-unit`                        | `bats --filter-tags 'unit,!integration' test/`.                                                                           |
| `just test-integration`                 | `bats --filter-tags integration test/`.                                                                                   |
| `just lint`                             | `pre-commit run --all-files`. Never invoke shellcheck/shfmt directly — always through pre-commit.                         |
| `just check`                            | `just lint && just test`. The gate CI runs.                                                                                |
| `just completions`                      | Copy `completions/claude-session.bash` to the target completions dir (honors user vs system install).                      |
| `just clean`                            | Remove generated artifacts (bats tempdirs). Does **not** touch anything installed outside the repo.                        |

Style conventions:

- Recipe names: lowercase, hyphenated. No camelCase.
- Each recipe has a single-line `#` doc-comment above it — that is
  what `just --list` shows.
- Destructive recipes carry an inline comment calling out the blast
  radius.

Example `just --list` output:

```
Available recipes:
    check                   # run lint + test (CI gate)
    clean                   # remove generated artifacts (non-destructive)
    completions             # install bash completions
    install PREFIX=""       # install claude-session (user scope if PREFIX empty)
    lint                    # run pre-commit on all files
    test                    # run unit + integration tests
    test-integration        # run integration bats tests
    test-unit               # run unit bats tests
    uninstall               # remove files listed in the install manifest
```

## Manifest-based uninstall

`install.sh` records every destination path into
`${XDG_STATE_HOME:-$HOME/.local/state}/claude-session/install-manifest`
(one absolute path per line). `uninstall.sh`:

1. Reads the manifest.
2. Removes each listed path in reverse order (files, then directories).
3. Only `rmdir`s directories — never `rm -rf` — so a user-created file
   dropped into an install dir prevents deletion and is preserved.
4. Removes the manifest itself last.
5. Does **not** touch `$XDG_CONFIG_HOME/claude-session/` (user
   config) or any `sessions/` subdir under
   `${XDG_STATE_HOME:-$HOME/.local/state}/claude-session/` — that
   subdir is the XDG-state fallback session root, populated only
   when `XDG_RUNTIME_DIR` was unavailable, and may contain live
   session state. The install manifest itself lives at
   `${XDG_STATE_HOME:-$HOME/.local/state}/claude-session/install-manifest`
   and is removed last.

If the manifest is missing, `uninstall.sh` logs a warning with the
expected path and exits 1 (per the standard exit-code contract in
[commands.md](commands.md)).

## Post-install setup

After `just install`, suggest adding to the user's shell rc:

```sh
# alias claude to the wrapper for automatic per-terminal isolation
alias claude='claude-session run'

# enable bash completion (if completions dir is not auto-sourced)
# shellcheck disable=SC1091
. "${XDG_DATA_HOME:-$HOME/.local/share}/bash-completion/completions/claude-session.bash"
```

Then:

```sh
claude-session config edit        # create your config.env interactively
claude-session doctor             # verify everything resolves
```

## PATH note

`$HOME/.local/bin` is the user-install bin directory. If it is not
already on `PATH`, `install.sh` prints a warning at the end with the
exact line to add to the user's shell profile. This mirrors the
behavior in the `devcontainerctl` reference installer.

## Upgrading

In-place upgrade: `git pull && just install`. `install.sh` overwrites
existing files at the same paths and updates the manifest in place.
Old files removed from the repo between versions are cleaned up by
comparing the new installation's output against the previous manifest.

For a clean slate, `just uninstall && just install`.
