# Development

Contributor / maintainer reference: how to set up the dev environment,
what the coding rules are, how tests are structured, and what the
release flow looks like. If you are writing code in this repo, read
this and [`architecture.md`](architecture.md) first.

## Prerequisites

- bash 4.4+ (for `inherit_errexit`; macOS' system bash is 3.2, install
  one from Homebrew).
- [`bats-core`](https://bats-core.readthedocs.io/) — test runner.
- [`shellcheck`](https://www.shellcheck.net/) — static analysis.
- [`shfmt`](https://github.com/mvdan/sh) — formatter.
- `jq`, `flock` — runtime deps, but tests rely on them too.
- `just` — recipe runner.
- [`pre-commit`](https://pre-commit.com/) — lint orchestration.

## Repo setup

```sh
git clone <url> claude-session
cd claude-session
pre-commit install                             # install the commit hook
just check                                     # lint + test sanity
```

## Strict mode policy

Every entry-point bash file (currently just `bin/claude-session`) uses:

```bash
set -euo pipefail
shopt -s inherit_errexit 2>/dev/null || true     # guarded for bash < 4.4
shopt -s failglob nullglob
```

Documented caveats to watch for (and that reviewers will flag):

- `set -e` is disabled inside `$(...)`, inside the LHS of `if` / `&&`
  / `||` chains, and inside `local var=$(…)` (the `local` masks the
  inner exit code). For load-bearing logic, prefer explicit `||
  cs::helpers::die` over trusting implicit `set -e`.
- `set -o pipefail` can turn a benign SIGPIPE (short-circuiting
  `grep -q`) into a failure. Use `|| true` where the pipe is
  intentionally short-circuited.
- **Do not set `IFS=$'\n\t'`** globally. Always quote `"$@"` and
  `"${arr[@]}"` and use arrays instead. This is a deliberate choice
  — see the "bash strict mode critique" references in
  [`architecture.md`](architecture.md) §"Strict mode".
- `trap '…' EXIT INT TERM` with single-quoted bodies (variables
  expand at trap time, not registration time).

Library files under `lib/` do **not** set strict mode themselves — the
entry point already did. They only need `# shellcheck shell=bash` at
the top.

## Shellcheck

`.shellcheckrc` at repo root:

```
external-sources=true
source-path=SCRIPTDIR
source-path=SCRIPTDIR/lib
shell=bash
```

Rules enforced on review:

- Every cross-file `source` carries an explicit
  `# shellcheck source=<path>` directive. Without it, shellcheck
  silently skips the sourced file and misses half the real bugs.
- Any `# shellcheck disable=SCxxxx` disable carries a one-line
  justification comment. Unexplained disables fail review.
- Warnings are errors in CI. Run `just lint` before pushing.

## File header convention

Every `.sh` under `lib/` starts with exactly two lines:

```bash
# shellcheck shell=bash
: 'desc: One-sentence description of what this file defines.'
```

- No shebang — these files are sourced, not executed.
- The `desc:` sentinel is harvested by the help/man generator (see the
  `usage` subcommand implementation) and by the `# <cmd>` doc-comment
  preamble in generated help output.
- The description must be a single line and describe the **intent**,
  not the implementation (e.g. "Dispatch a subcommand by sourcing the
  matching file", not "Source lib/commands/cmd_$1.sh").

## Naming convention

| Path                         | Defines                            | Visibility |
|------------------------------|------------------------------------|------------|
| `bin/claude-session`         | Entry shim; sets up env, calls `cs::main`. | n/a    |
| `lib/commands/cmd_<n>.sh`    | `cs::cmd::<n>`                     | public     |
| `lib/functions/fn_<n>.sh`    | `cs::fn::<n>`                      | public     |
| `lib/helpers.sh`             | `cs::helpers::*` utilities         | shared     |
| `lib/loader.sh`              | `cs::loader::dispatch`             | internal   |
| `lib/core.sh`                | `cs::main`                         | internal   |
| (any file) `__<n>`           | Private, same-file only            | private    |

**One public function per file** under `lib/commands/` and
`lib/functions/`, with the filename mirroring the function name. The
dispatcher (`cs::loader::dispatch`) derives the file path from the
subcommand without a lookup table: `sub` → `lib/commands/cmd_${sub}.sh`.

## Pre-commit hooks

Hook list for `.pre-commit-config.yaml` (derived from the bash subset
the repo owner uses across their other bash CLIs):

- **standard** (`pre-commit/pre-commit-hooks`):
  `trailing-whitespace`, `end-of-file-fixer`, `check-merge-conflict`,
  `check-yaml`, `check-executables-have-shebangs`,
  `check-shebang-scripts-are-executable`.
- **shellcheck** (`koalaman/shellcheck-precommit` or `shellcheck-py`):
  `--severity=warning`.
- **shfmt** (`mvdan/sh` pre-commit mirror):
  `-i 2 -ci -w` (2-space indent, compact if-else, write in place).

Run locally:

```sh
just lint              # pre-commit run --all-files
```

Never invoke `shellcheck`, `shfmt`, or any other linter directly —
always through `pre-commit`. This keeps one source of truth for config
and prevents drift between developers.

## Tests

### Layout

```
test/
├── test_helper.bash                      # common setup, sets PATH
├── cmd_run_test.bats
├── cmd_doctor_test.bats
├── cmd_config_test.bats
├── cmd_profile_test.bats
├── cmd_session_test.bats
├── fn_real_claude_test.bats
├── fn_session_dir_test.bats
├── fn_compose_profile_test.bats
├── fn_terminal_id_test.bats
└── fn_sync_files_test.bats
```

Filename convention: `<module>_test.bats` (suffix, not prefix).

### `test_helper.bash`

```bash
_common_setup() {
  PATH="${BATS_TEST_DIRNAME}/../bin:$PATH"
}
```

Per-test setup:

```bash
setup() {
  load 'test_helper'
  _common_setup
}
```

### Tagging unit vs integration

Bats supports inline tags:

```bash
# bats test_tags=unit
@test "fn_terminal_id handles missing tty" {
  …
}

# bats test_tags=integration
@test "run creates session dir and invokes claude" {
  …
}
```

- **Unit** tests: no filesystem side effects beyond `BATS_TEST_TMPDIR`,
  no subprocess calls to the real `claude` binary. Fast; run on every
  commit.
- **Integration** tests: may create real session dirs, merge settings
  via `jq`, exercise the full dispatch path. Use a mocked `claude`
  stub via `$CLAUDE_SESSION_REAL_CLAUDE` pointing at a test-local
  fake binary.

Run via `just test-unit`, `just test-integration`, or `just test` for
both.

## CI matrix

Minimum viable GitHub Actions matrix:

```yaml
jobs:
  check:
    strategy:
      matrix:
        bash: ['4.4', '5.0', '5.2']
    steps:
      - uses: actions/checkout@v4
      - run: just lint
      - run: just test
```

One job per bash version. The single gate is `just check`.

## Commit style

[Conventional Commits](https://www.conventionalcommits.org/). Scopes
should map to either a subcommand (`cmd_doctor`, `cmd_run`), a function
(`fn_real_claude`), or an area (`install`, `docs`, `ci`). Examples:

```
feat(cmd_session): add --older-than duration flag
fix(fn_real_claude): skip PATH entries that resolve back to wrapper
docs(config): document CLAUDE_SESSION_SYNC_FILES default
chore(ci): add bash 5.2 to matrix
```

Breaking changes carry a `!` and a `BREAKING CHANGE:` footer.

## Agent-surface checklist

Applied during PR review for any change touching subcommands:

- [ ] Each subcommand has complete `--help` (flags, defaults, examples,
      exit codes, `SEE ALSO`).
- [ ] `usage` aggregate output regenerates cleanly.
- [ ] Error paths emit the three-part shape
      (`What went wrong:` / `How to fix:` / `Next:`).
- [ ] Exit codes match the table in
      [`commands.md`](commands.md) §"Exit codes".
- [ ] `doctor` covers any new external dependency or env var.
- [ ] stdout stays parseable; all logging / progress goes to stderr.
- [ ] No ANSI escape codes unless `[[ -t 1 ]]`.
- [ ] Config precedence is `flags > env > file`.
- [ ] No interactive prompt when `[[ ! -t 0 ]]` — fail with remediation.
- [ ] Destructive ops have `--dry-run` and require `--yes` when stdin
      isn't a tty.

## Release flow

1. Tag `vX.Y.Z` on `main`, following semver.
2. CI runs `just check` against the tag on all supported bash versions.
3. Update the `CHANGELOG.md` (Keep a Changelog format) before cutting
   the tag.
4. Publish a GitHub release with the changelog entry as the body.

Package distribution (Homebrew formula, AUR `PKGBUILD`, Nix flake,
`.deb`) is deliberately out of scope for v1; v1 ships as
multi-file + `install.sh` via `just install`. Distribution targets are
tracked as issues and picked up post-v1.

## Style references

Three external documents shape the style in this repo. Read them if
you are doing non-trivial work:

- The internal bash-CLI layout reference — directory shape, strict
  mode, ShellCheck discipline, bats layout, XDG install.
- The internal agent-CLI design reference — per-subcommand `--help`,
  `usage`, `doctor`, error shape, config precedence, stdout/stderr
  discipline, exit codes.
- The internal bash-package `CLAUDE.md` — one-public-function-per-file
  pattern, `__` prefix for privates, linting through pre-commit only.

These are also summarized in [`AGENTS.md`](../AGENTS.md).
