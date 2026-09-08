# Testing and quality

Test tools, test lanes, the gate map, and which contract each mandatory test locks down. The approach and its rationale are in [the testing strategy](../explanation/testing-strategy.md).

The native passthrough, secure session storage, full profile composition, doctor, presentation, login-account MVP, generated-artifact, and generated-example rows are implemented. Remaining later-slice rows stay normative design until their owners land.

## Test kinds

| Kind        | Location                                  | `nextest` filter                                      | What it tests                                                         |
| ----------- | ----------------------------------------- | ----------------------------------------------------- | --------------------------------------------------------------------- |
| Unit        | Inline `#[cfg(test)] mod tests` in `src/` | `kind(bin)`; `kind(lib)` once a library target exists | One function or module in isolation                                   |
| Integration | `tests/*.rs`                              | `kind(test)`                                          | The seam between components, against a stub child or a temporary tree |
| End-to-end  | Continuous integration only               | —                                                     | The whole product against the real `claude`                           |
| Doc tests   | `///` examples                            | Not run by `nextest`                                  | Runs against the tooling-facing library target                        |

The defining property of an integration test is that it tests an interaction, not that it touches something real. A test running the compiled binary against a recording stub is an integration test.

End-to-end tests never run in a local hook. They are slow, they depend on host state, and they need credentials. A contributor must be able to run the whole local suite with nothing installed but the toolchain.

## Lanes

| Lane                   | Profile      | Runs                     | Budget               |
| ---------------------- | ------------ | ------------------------ | -------------------- |
| `pre-commit`           | `pre-commit` | Unit only                | Under one second     |
| `pre-push`             | `pre-push`   | Unit and integration     | Single-digit seconds |
| Continuous integration | `ci`         | Everything, with retries | Minutes              |

The unit lane is wired to both the commit and push hooks. It is nearly free, and running it again at push catches work that reached a commit through a rebase or an amend that bypassed the hook.

The profile foot-gun: a non-default `nextest` profile is inert unless the invoking command passes `--profile <name>`. A hook or recipe that omits it silently runs the default profile — every test, with the wrong settings — and the lane's filter never applies. Every invocation names its profile explicitly.

The time budget is one number: the commit hook stays under one second of test time. A commit hook slow enough to notice is a commit hook people bypass.

### What each lane may admit

A lane is defined by the evidence it is allowed to look at, which is what keeps a slow test from drifting into a fast lane:

| Lane        | Admissible evidence                                                                                                                                                                                                                                                        |
| ----------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Unit        | The return value of a function called in-process. No process, no filesystem, no clock, no environment.                                                                                                                                                                     |
| Integration | The compiled binary's exit status, its standard output and standard error as bytes, the three raw-byte files [the recording stub](#the-recording-stub) wrote, and the bytes of a checked-in repository file read read-only under the manifest directory. Never host state. |
| End-to-end  | Observations of the real `claude`. The only lane that may, and it may never run in a hook.                                                                                                                                                                                 |

A checked-in repository file is admissible because it is versioned and deterministic and changes only with a commit, so a test reading one is evidence about this repository rather than about the host. Reading it still costs the filesystem access the unit lane forbids, which is what puts a documentation gate in the push lane rather than the commit lane. That admission is a lane rule and not a fourth lane, because a fourth lane would need its own profile, hook row, and budget to discriminate nothing the integration lane does not already discriminate.

The lane of a mandatory test is derived, never declared: a test needing the recording stub, a temporary tree, or a checked-in repository file outside the crate's sources is integration, one needing the real child is end-to-end, and everything else is unit. That rule is total over the table below, which is why no test carries a lane column — a column would be a second source of truth over forty-odd rows, and the first row to disagree with the rule would be a defect nobody could see.

## Tools

| Tool            | Role                                                                           |
| --------------- | ------------------------------------------------------------------------------ |
| `cargo nextest` | Test runner. Per-profile filters, and it suppresses output from passing tests. |
| `assert_cmd`    | Runs the compiled binary and asserts on status, stdout, stderr                 |
| `predicates`    | Composable assertions                                                          |
| `insta`         | Snapshots, for help output and structured reports                              |
| `assert_fs`     | Filesystem fixtures and assertions                                             |
| `tempfile`      | Per-test temporary directories                                                 |
| `proptest`      | Property tests — notably argv round-tripping                                   |
| `cargo mutants` | Mutation testing, advanced tier                                                |
| `cargo bloat`   | Binary size attribution, advanced tier                                         |

Advanced-tier tools run on demand, not in a lane. They are diagnostics for a specific question, and gating on them buys noise.

`predicates` string matchers and `insta::assert_snapshot!` are UTF-8 only, so neither may carry an argv or environment assertion — a lossy conversion inside the harness would let a broken wrapper pass. Where a snapshot of raw bytes is wanted, `insta::assert_binary_snapshot!` is the one that keeps them.

## Hermetic fixtures

Every test touching the environment or the filesystem must satisfy all of these:

| Requirement          | Rule                                                                                                                      |
| -------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| Temporary directory  | Fresh per test, removed after. Never shared.                                                                              |
| Child environment    | Cleared, then explicitly populated. Never inherited and patched.                                                          |
| Base directories     | Every `XDG_*` variable points inside the temporary directory                                                              |
| Network              | None                                                                                                                      |
| Clock                | Injected where a timestamp is observable                                                                                  |
| Test-process globals | Never mutated. No environment mutation in the test process; no changing the working directory.                            |
| Controlling terminal | Never the one the suite was started from. A prompting verb runs with `--yes`, detached, or on its own allocated terminal. |

The controlling terminal is not a stream, so clearing the environment and capturing standard input do not take it away: a confirmation addresses `/dev/tty` and finds whichever terminal the developer started the suite from. A test that invokes `account remove`, `account login --token`, or `session clean` without `--yes` therefore asks a real person and waits — passing under a runner with no terminal and hanging a `git push` on a workstation. The fixture must decide it: `detached_command` runs under `setsid` so there is no controlling terminal to find, and `terminal_command` allocates one of its own so the answer comes from the test.

The last row above is the one that produces the worst bugs. Both the environment and the working directory are shared across a parallel test runner, so mutating either corrupts unrelated tests roughly one run in twenty — a failure rate that trains people to re-run rather than read.

## The recording stub

Real-process tests use a stub binary placed on the search path ahead of anything else. It records the arguments, environment, and working directory it received, and exits with whatever status the test requires — including death by a chosen signal.

It records raw bytes, in the kernel's own format. Three files in a directory the test names by variable — `argv`, `environ`, `cwd` — each entry written verbatim and separated by a NUL, with a trailing NUL. That is the `/proc/<pid>/cmdline` and `/proc/<pid>/environ` layout, and it is lossless without a length prefix because a NUL cannot occur inside an argument or an environment entry. Text, JSON, and any lossy conversion are forbidden: a stub that normalizes makes the golden argv test pass against a broken wrapper. Files rather than standard output, because the child's stdout is inherited unmodified and is itself under test.

The stub is what makes passthrough assertions mechanical: not "the command looked right" but "the child received exactly these arguments, in this order, with these bytes."

## Mandatory tests

Each of these locks down a contract that is otherwise decorative:

| Test                      | Locks                                                                                                     | Owning document                               |
| ------------------------- | --------------------------------------------------------------------------------------------------------- | --------------------------------------------- |
| Golden argv table         | The whole vector: `argv[0]`, the wrapper prefix, and a suffix preserved in order, count, and bytes        | [CLI surface](./cli-surface.md)               |
| Exit-code matrix          | Every error variant maps to its documented code, no catch-all                                             | [Exit codes](./exit-codes.md)                 |
| Child exit fidelity       | A stub exiting with N produces N                                                                          | [Exit codes](./exit-codes.md)                 |
| Child signal fidelity     | A signal-killed stub produces genuine signal death, never an encoded exit                                 | [Exit codes](./exit-codes.md)                 |
| `--` sentinel             | A wrapper flag after `--` reaches the child uninterpreted                                                 | [CLI surface](./cli-surface.md)               |
| Recursion guard, marker   | The marker variable stops re-entry, including a nested Claude Code session                                | [Process runtime](./process-runtime.md)       |
| Recursion guard, identity | A hard-linked wrapper is caught, which path equality would miss                                           | [Process runtime](./process-runtime.md)       |
| Terminal ladder           | A `child_bin` naming a missing file exits 127 and never falls through to `PATH`                           | [Process runtime](./process-runtime.md)       |
| `PATH` search rules       | A zero-length entry is skipped; a permission-rejected candidate decides 126                               | [Process runtime](./process-runtime.md)       |
| Exec-failure classes      | A child removed after the pre-flight check exits 127, not `OsError`                                       | [Process runtime](./process-runtime.md)       |
| Flush before the exec     | The boundary flushes the log sink before it replaces the image, asserted in order rather than by timing   | [Process runtime](./process-runtime.md)       |
| Environment isolation     | The stub sees the injected config directory and exactly one `CLAUDE_SESSION_*` key, the marker            | [Process runtime](./process-runtime.md)       |
| Environment fidelity      | A non-UTF-8 ambient variable reaches the stub unchanged, and no wrapper input does                        | [Process runtime](./process-runtime.md)       |
| Profile isolation         | Two profiles launched from one terminal and one account get different entry paths and bytes               | [XDG storage](./xdg-storage.md)               |
| Entry key determinism     | Changing a piece's content, resolved path, order, or the strategy table names a different entry           | [XDG storage](./xdg-storage.md)               |
| Terminal independence     | Identical inputs under different terminal state and different accounts name the same entry                | [XDG storage](./xdg-storage.md)               |
| Entry immutability        | An existing entry is never rewritten, and a run that finds a matching one composes nothing                | [XDG storage](./xdg-storage.md)               |
| Sidecar mismatch refusal  | An entry whose recorded digest disagrees with the recomputed one is neither opened nor overwritten        | [XDG storage](./xdg-storage.md)               |
| Partial pair recovery     | With exactly one member present, both are written from this run's inputs, never the survivor kept         | [XDG storage](./xdg-storage.md)               |
| Symlink rejection         | A wrapper-managed path that is a symlink is refused                                                       | [XDG storage](./xdg-storage.md)               |
| Mode enforcement          | An over-permissive directory is corrected, and the check reports `pass`, not `fail`                       | [XDG storage](./xdg-storage.md)               |
| Unmanaged ancestors       | A `0755` `$HOME` or `.local` is never checked or corrected                                                | [XDG storage](./xdg-storage.md)               |
| Interrupted write         | An abandoned temporary leaves the previous complete file readable at the final path                       | [XDG storage](./xdg-storage.md)               |
| Sweep safety              | An orphaned temporary is removed, and one whose process id is live is kept                                | [XDG storage](./xdg-storage.md)               |
| Cross-process exclusion   | A second writer of a locked scope waits, then exits `LockBusy` at its deadline                            | [XDG storage](./xdg-storage.md)               |
| In-process exclusion      | Two threads writing one scope serialize, which the file lock alone would not achieve                      | [XDG storage](./xdg-storage.md)               |
| Lock release on death     | A holder killed by `SIGKILL` leaves the next acquisition uncontended                                      | [XDG storage](./xdg-storage.md)               |
| Unknown configuration key | A typo is rejected, naming the key and file                                                               | [Configuration](./configuration.md)           |
| Merge determinism         | The same pieces produce byte-identical output                                                             | [Configuration](./configuration.md)           |
| Freshness on piece change | Editing a piece without the profile names a new entry and leaves the old one untouched                    | [Configuration](./configuration.md)           |
| Example round-trip        | Every generated example parses through the real loader                                                    | [Configuration](./configuration.md)           |
| Undocumented field        | A public config field without a description fails generation                                              | [Configuration](./configuration.md)           |
| Check-id coverage         | Every catalog id maps to an `err.kind` that exists                                                        | [Doctor](./doctor.md)                         |
| Doctor projection parity  | Human, JSON, and both list modes preserve the same catalog ids and metadata                               | [Doctor](./doctor.md)                         |
| Doctor traversal and exit | Failures do not abort later safe probes; first hard failure and strict promotion remain ordered           | [Doctor](./doctor.md)                         |
| Doctor summary and skips  | Counts cover every public row once and inapplicable session subjects carry neutral reasons                | [Doctor](./doctor.md)                         |
| Doctor child composition  | A failed child report fails the verdict without `--strict`, and a wrapper hard failure outranks its code  | [Doctor](./doctor.md)                         |
| Doctor remediation parity | A guard and the report use the catalog-owned remediation without paraphrase                               | [Doctor](./doctor.md)                         |
| Help snapshot             | Generated help does not change unnoticed                                                                  | [CLI surface](./cli-surface.md)               |
| Generated artifacts       | Completions and the man page carry the implemented wrapper grammar and leak no child flag                 | [CLI surface](./cli-surface.md)               |
| Denylist membership       | The spellings the pre-split claims are exactly the documented table                                       | [CLI surface](./cli-surface.md)               |
| Spelling matrix           | Exact matching: no abbreviation, no bundling, no case folding, both value forms                           | [CLI surface](./cli-surface.md)               |
| Leading-position scope    | A claimed flag after any other token reaches the child                                                    | [CLI surface](./cli-surface.md)               |
| Malformed wrapper flag    | A claimed flag missing its value exits `Usage`; a near-miss forwards                                      | [CLI surface](./cli-surface.md)               |
| Collision audit           | The claimed set meets the child's inventory only where documented                                         | [CLI surface](./cli-surface.md)               |
| Child version floor       | A `login`-mode launch below the floor fails before the exec; `token` mode does not                        | [Process runtime](./process-runtime.md)       |
| Account discovery         | Valid account directories are listed without a registry index                                             | [Accounts](./accounts.md)                     |
| Native login delegation   | Login delegates exact child argv and failed first-login cleanup cannot escape its new account tree        | [Accounts](./accounts.md)                     |
| Account local usability   | List derives usability from local metadata and path probes without spawning a child                       | [Accounts](./accounts.md)                     |
| Account marker ordering   | The selected account marker is visible to the final child before exec                                     | [Process runtime](./process-runtime.md)       |
| Mandatory session binding | Missing axes are named as `Config`/78 with concrete hints, and no child invocation occurs                 | [Process runtime](./process-runtime.md)       |
| Account grammar and help  | Only leading `account login` and `account list` are claimed, with required subcommand and sentinel escape | [CLI surface](./cli-surface.md)               |
| Confirmation predicate    | A piped invocation with a controlling terminal still prompts                                              | [CLI surface](./cli-surface.md)               |
| Confirmation escape       | With no controlling terminal, `--yes` removes and its absence exits `Unavailable`                         | [CLI surface](./cli-surface.md)               |
| ADR contract              | Record id, shape, status vocabulary, relationship links, and the word cap                                 | [Documentation sweeps](#documentation-sweeps) |
| Plan-zone contract        | Milestone rows, slice shape, appetite agreement, EARS acceptance, and the question register               | [Documentation sweeps](#documentation-sweeps) |
| Decorative emphasis       | No bold or italic prose outside code, over every document in the tree                                     | [AGENTS.md](../../AGENTS.md)                  |
| Boundary facts            | `domain/` names no adapter or service, `src/` names no `xtask`, the manifest lists no tooling crate       | [Boundary lints](#boundary-lints)             |

### Naming the implementation a test rejects

A mandatory test names the wrong implementation it rejects only where a naive implementation would pass the obvious assertion. Most rows do not need one: a test that asserts the documented behaviour already fails everything else. Spelling out a rejected implementation for all of them would be table-completeness rather than coverage, and each sentence would then have to be maintained against code that does not exist yet.

These are the rows where the naive implementation passes and the contract still breaks:

| Test                      | Passes naively, but is wrong                                                                                                                           |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Recursion guard, identity | Comparing canonical paths. A hard link to the wrapper is a different path and the same file.                                                           |
| Exec-failure classes      | Classifying a child removed between the pre-flight check and the exec as `OsError`. The pre-flight check is advisory, so the exec's own errno decides. |
| Flush before the exec     | Reading the log file after a real launch. The worker thread usually wins that race, so the assertion passes with the flush deleted.                    |
| Mode enforcement          | Reporting `fail` on a mode that was corrected. A repair is not an unhealthy state, so `doctor --strict` must not fail on one.                          |
| Partial pair recovery     | Keeping the surviving member of a half-written pair. Both are rewritten from this run's inputs, because the survivor's provenance is unknown.          |
| In-process exclusion      | Relying on the file lock alone. An advisory lock is held per open file description and cannot exclude a second thread of the same process.             |

The three tables below apply that rule at length, because each covers an obligation split across several assertions.

The golden argv table is the proof of [ADR-0002](../decisions/ADR-0002-verbatim-argv-passthrough.md), so its legs are named rather than left to judgement:

| Leg                     | Shape                                                                                              |
| ----------------------- | -------------------------------------------------------------------------------------------------- |
| `argv[0]`               | The stub's own absolute resolved path, not a bare name and not the wrapper's.                      |
| Empty argument          | `""` arrives as a real argument, in position, not filtered.                                        |
| Non-UTF-8 argument      | The bytes `[0x66, 0x80, 0x6f]` — a lone continuation byte, invalid UTF-8, legal in an argument.    |
| Non-UTF-8 settings path | `XDG_STATE_HOME` pointed at a directory with those bytes; the two-token prefix arrives byte-exact. |
| Around `--`             | A wrapper spelling after the sentinel arrives uninterpreted, and count is preserved.               |

Every leg compares OS strings against OS strings. A test that renders either side as text has stopped testing the contract, which is why the stub records bytes.

The five flag-recognition tests are one obligation split by what each rejects, and together they are the proof of [ADR-0043](../decisions/ADR-0043-match-wrapper-flags-by-exact-leading-spelling.md) and [ADR-0044](../decisions/ADR-0044-audit-wrapper-spellings-against-the-child-inventory.md):

| Test                   | Shape                                                                                                                                           |
| ---------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| Denylist membership    | A snapshot of the sorted claimed-spelling set, derived from the parser rather than hand-written.                                                |
| Spelling matrix        | Table-driven over every claimed flag by `--x=v`, `--x v`, `--x -y`, `-X`, `-XY`, an uppercase spelling, and a unique prefix.                    |
| Leading-position scope | The stub receives `--verbose` from `claude-session -p x --verbose`.                                                                             |
| Malformed wrapper flag | `--config` bare exits `Usage`; `--configg` forwards verbatim and the run exits with the stub's status.                                          |
| Collision audit        | The claimed flag and verb sets are intersected with a checked-in, version-labelled inventory fixture and compared with the documented overlaps. |

The collision audit reads the fixture, never the network and never a locally installed child; refreshing the fixture is the `child-flag-and-verb-inventory` revalidation, not a test run. It lives in `tests/collision_audit/` and compares [the child inventory](../../tests/fixtures/child-inventory.yaml) against the three tables in [the CLI surface](./cli-surface.md) that claim spellings and resolve overlaps.

Its own integrity is the harder half. A documentation gate that silently matches zero rows reports success, so the audit proves it found what it was aiming at before it compares anything: the table headers must match exactly and in order, row counts must clear a floor, and named rows must be recovered — the value placeholder in `--config <path>` must strip to one spelling, and the `--version`, `-V` row must yield exactly one colliding and one free spelling. An unrecognized child-status clause is a failure rather than a default, because defaulting to free is precisely the collision the audit exists to catch. Negative cases drive the comparison from doctored literals, so a run that goes green has demonstrated it can go red.

The two confirmation tests exist to reject one specific wrong implementation — `stdin().is_terminal()`, which passes a naive suite and fails only where the two predicates disagree ([ADR-0053](../decisions/ADR-0053-read-a-confirmation-from-the-controlling-terminal.md)):

| Test                   | Shape                                                                                                                                                                                      |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Confirmation predicate | A pseudo-terminal is the child's controlling terminal, and fd 0 is a pipe. Answering `n` on the master exits `0`, leaves the account directory intact, and consumes nothing from the pipe. |
| Confirmation escape    | The child is detached with `setsid`. Without `--yes` it exits `Unavailable` with the directory byte-for-byte unchanged; with `--yes` it removes.                                           |

Each carries its `y`, `yes`, bare-Enter, and end-of-input legs, and one `--json` leg asserting exactly one document on standard output while the prompt went to the terminal.

A confirmation test must detach or allocate its own terminal — never inherit the runner's. Under the `/dev/tty` predicate an inherited terminal makes the binary prompt at whoever ran `cargo test`, so the suite hangs locally and passes in CI, which is the worst failure shape a gate can have. This is the [test-process globals](#hermetic-fixtures) rule reaching the terminal.

`rustix` supplies `process::setsid`, the `pty` module, and the `termios` calls that [ADR-0027](../decisions/ADR-0027-ingest-secrets-only-from-stdin-or-a-terminal.md)'s echo-disable needs; `TIOCSCTTY` goes through `rustix::ioctl` or the `libc` exception. No terminal-scraping crate is added — see [dependencies](./dependencies.md).

Five of these have teeth beyond their own assertion. The exit-code matrix, written exhaustively over a closed enum, means adding an error variant without a code fails the build. The example round-trip is what stops a generated example from being a plausible-looking file the program itself would reject — an example that does not parse is worse than none, because the user trusts it. The undocumented-field test enforces the hard failure [ADR-0013](../decisions/ADR-0013-generate-config-examples-from-types.md) rests on: without it, the generator degrades quietly into emitting bare keys. Denylist membership means a flag added in code without its table row fails the build, and the collision audit means a child release that starts shadowing a claimed spelling fails the build — the only mechanism in the project that turns red without a change of its own, which is the point.

## The gate

Hooks are the source of truth; task-runner gate recipes delegate to them, while inner-loop recipes stay raw `cargo`.

`just hooks` is the single local command that reproduces the project's verdict. It runs both stages:

```bash
pre-commit run --all-files --hook-stage pre-commit
pre-commit run --all-files --hook-stage pre-push
```

`pre-commit run --all-files` on its own is not the gate. `--all-files` selects files, not stages, so it runs the commit stage alone and silently omits the push-stage half of the table below — the integration tests, the doctests, and every advisory and secret scan.

Run it inside the devShell. Several hooks take their binary from the shell rather than building one, so outside it they fail at exec rather than reporting on content ([ADR-0040](../decisions/ADR-0040-provision-hook-binaries-from-the-devshell.md)).

The Backing column says whether the hook exists today. `deferred` means the row is a specification the repository does not yet enforce; it is closed by the round that builds the mechanism, never by deleting the row.

| Hook                                             | Stage        | Enforces                                       | Backing                                                                   |
| ------------------------------------------------ | ------------ | ---------------------------------------------- | ------------------------------------------------------------------------- |
| `cargo fmt`                                      | commit       | Canonical formatting                           | present                                                                   |
| `clippy` auto-fix, then gate                     | commit       | Lint clean, warnings as errors                 | present                                                                   |
| `cargo nextest` (`pre-commit` profile)           | commit, push | Unit tests                                     | present                                                                   |
| `cargo nextest` (`pre-push` profile)             | push         | Integration tests                              | present                                                                   |
| `cargo test --doc`                               | push         | Doctests, guarded on a library target existing | present                                                                   |
| `taplo`                                          | commit       | TOML formatting                                | present                                                                   |
| `typos`                                          | commit       | Spelling                                       | present                                                                   |
| `ripsecrets`                                     | commit       | Fast secret scan                               | present                                                                   |
| `gitleaks`                                       | push         | Full secret scan                               | present                                                                   |
| `cargo audit`                                    | push         | Advisories                                     | present                                                                   |
| `cargo deny`                                     | push         | Advisories, bans, sources, licences            | partial — see [dependencies](./dependencies.md#lockfile-and-supply-chain) |
| `cargo machete`                                  | push         | Unused dependencies                            | present                                                                   |
| `cargo xtask gen-config`                         | commit       | Generated examples match the config types      | present                                                                   |
| `dprint`                                         | commit       | Markdown and JSON formatting                   | present                                                                   |
| `markdownlint-cli2`                              | commit       | Markdown structure and link integrity          | present                                                                   |
| `md-slice-readme`, `md-milestones`, `md-adr`     | commit       | Fixed heading shapes, one array per shape      | present                                                                   |
| `shellcheck`, `shfmt`                            | commit       | Shell scripts                                  | present                                                                   |
| `nixfmt`, `statix`, `deadnix`                    | commit       | Nix sources                                    | present                                                                   |
| `no-commit-to-branch`                            | commit       | No direct commit on `master`                   | present — in the release-kit block                                        |
| `rk-branch-name`, `rk-worktree-location`         | commit       | Branch naming and the worktree rule            | present — in the release-kit block                                        |
| `conventional-pre-commit`                        | commit-msg   | Conventional Commits, scope required           | present — in the release-kit block                                        |
| `rk-message`                                     | commit-msg   | Message content guards                         | present — in the release-kit block                                        |
| `rk-status-check`                                | commit       | The landed release payload is undrifted        | present — in the release-kit block                                        |
| `rk-no-push-to-trunk`, `rk-no-hand-authored-tag` | push         | The trunk and tag invariants                   | present — in the release-kit block                                        |

This table lists the gates the specifications depend on, not every hook configured. The file-hygiene hooks — private-key detection, symlink and large-file checks, JSON5 and editorconfig validation — are configured and depend on no specification, so they carry no row.

Fast, autofixing checks run at commit; slow and network-dependent ones at push. Do not bypass a hook. A hook that is wrong should be fixed in its configuration.

Continuous integration runs a subset, not the whole gate. `ci.yml` invokes the task-runner recipes — formatting, clippy, the `ci` test profile, a release build, docs, `cargo audit`, `cargo deny`, and `nix flake check` — and never invokes `pre-commit`. Everything else in the table is enforced locally only and can therefore reach a green pull request unrun. Closing that is a workflow change, and until it lands this page does not claim the two are equivalent.

## Boundary lints

Four architectural rules are structural rather than type-checked. Two ban a Rust API and are enforced by clippy configuration; two are module-graph and manifest facts that no lint can express, so they are integration tests that read the tree ([ADR-0074](../decisions/ADR-0074-enforce-boundary-rules-with-clippy-configuration.md)).

| Rule                 | Mechanism                                                          | Backing |
| -------------------- | ------------------------------------------------------------------ | ------- |
| Output ownership     | `clippy.toml` `disallowed-macros`                                  | present |
| Environment typing   | `clippy.toml` `disallowed-methods`, replacement `std::env::var_os` | present |
| Dependency direction | `tests/repo_contracts/boundaries.rs`                               | present |
| Tooling isolation    | `tests/repo_contracts/boundaries.rs`, plus `cargo-deny` `bans`     | present |

Output ownership. No print macro appears in `src/` outside the output module and the entry point. See [logging and output](./logging-and-output.md#the-stream-contract).

Environment typing. `std::env::var` and `std::env::vars` are banned under `src/`; the `_os` forms only. Both panic on an environment that is not valid UTF-8, which would turn a legal environment into a wrapper crash — the same conversion the [types rule](./coding-conventions.md#types) forbids on argv, in the place a type cannot catch it.

Dependency direction. `domain/` imports nothing from `adapters/` or `services/`. A violation means pure code has acquired an I/O dependency, and the type system will not catch it.

Tooling isolation. Nothing under `src/` imports from `xtask`, and the wrapper's own manifest does not list a development-tooling crate. The dependency runs one way, and the whole reason `xtask` exists is that its dependencies stay out of the shipped binary; see [dependencies](./dependencies.md).

Scope all but tooling isolation to `src/`. The two clippy rules resolve paths, so they neither fire inside a `///` example nor miss an aliased import — which is the whole reason they are not lints over text. The two that remain text scans carry that hazard and are written to tolerate it: the `xtask` scan tests word boundaries, so it rejects `xtask::` and accepts `my_xtask`, and the manifest half parses `[dependencies]` rather than matching lines, so a `[dependencies.name]` subtable cannot slip past.

The two clippy rules are wired at pre-commit and pre-push; the two facts are tests, so they run in the pre-push integration lane. A boundary violation is therefore caught at push rather than at commit, which is the price of the lane rule at [what each lane may admit](#what-each-lane-may-admit): anything reading a checked-in file costs the filesystem access the commit lane's one-second budget forbids. Implemented slice acceptance may append an exact nextest ID after `->`; `scripts/check-acceptance-tests` rejects duplicate or unresolved IDs at pre-push.

## Protected branches

The gate refuses a commit made directly on `master`, and takes no position on `develop`. `master` is written by the installed GitHub App alone ([release workflow](./release-workflow.md#branch-and-release-invariant)), so a local commit there has no legitimate case and the hook rejects that class with no false positives.

`develop` is deliberately excluded. Its real policy is a reviewed pull request with green continuous integration, which a client-side hook cannot approximate and would only imitate — and it has one legitimate direct-commit case, during [release bootstrap](../guides/releasing.md#bootstrap-release-automation-once). The forge ruleset is its authority. The general rule: local hooks validate content, forge rules enforce branch topology.

The hook is specified and not yet enabled. `no-commit-to-branch` is commented out in the hook configuration, so this rule is enforced by review until it is uncommented. Enabling it before the bootstrap is done would reject the direct commits the release guide requires, so its precondition is a published `develop` and a closed bootstrap window.

It must carry `args: [--branch, master]`. The hook's default set is `master` and `main`; only `master` is protected here, and the explicit argument keeps the hook from asserting a branch this project's model does not name.

## Documentation sweeps

The test suite is the enforcement lookup. Three integration gates reject ADR-contract drift, plan-zone drift, and decorative emphasis; markdownlint owns structural Markdown checks. Review sweeps cover facts that need human classification.

Each gate carries its own integrity proof, for the reason the collision audit does: a gate that matches zero rows reports success. The ADR gate floors the record count and names sentinel ids, the plan-zone gate requires a recovered milestone line for every slice directory, and the emphasis gate floors the file count, names sentinel paths, and asserts that build output was excluded rather than merely absent. Negative fixtures drive every rule from a doctored literal, so a run that goes green has demonstrated it can go red.

| Check                                                                  | Rejects                                                                       |
| ---------------------------------------------------------------------- | ----------------------------------------------------------------------------- |
| [`repo_contracts::adrs`](../../tests/repo_contracts/adrs.rs)           | ADR id, shape, status, relationship, or 350-word-cap drift.                   |
| [`repo_contracts::plan_zone`](../../tests/repo_contracts/plan_zone.rs) | Slice, milestone, task, acceptance, rabbit-hole, and question-contract drift. |
| [`repo_contracts::emphasis`](../../tests/repo_contracts/emphasis.rs)   | Unapproved bold or italic prose outside code.                                 |
| `markdownlint-cli2`                                                    | Invalid Markdown, broken relative links, and unlabelled fences.               |
| `md-*` heading-shape hooks                                             | A fixed shape gaining, losing, or reordering a heading.                       |
| [`repo_contracts::shape`](../../tests/repo_contracts/shape.rs)         | The two shape-wiring faults those hooks report as success.                    |
| Research-tracking inspection                                           | Missing six-field entries or paths that no longer resolve.                    |
| Personal-path and marker sweeps                                        | Load-bearing external paths or unresolved promises under `docs/`.             |

```bash
cargo nextest run --profile pre-push --all-features -E 'binary(repo_contracts)'

rg -n '(/h[o]me/|/U[s]ers/|~[/]|file:/{2}|exobrain-[t]ech)' docs AGENTS.md README.md \
  .pre-commit-config.yaml scripts

rg -n -i '\b(TOD[O]|TB[D]|FIXM[E]|XX[X])\b' docs --glob '!research-tracking.yaml'
```

Two personal-path hits are declared examples rather than defects: the rendered provenance sidecar in [configuration](./configuration.md#provenance-sidecar), where the absolute path is the field value, and quotations of the XDG specification's own home-relative defaults where that default is the specified value. No other personal or external local path may carry authority.

Counting rule: never combine `grep -c` with `-o`. Count occurrences with `grep -ohE PATTERN FILE | sort -u | wc -l`, which de-duplicates and behaves the same on every host.

## Markdown

Documentation passes the same gate as code.

| Constraint                          | Consequence for authoring                                                                                                                                                                                                                                                |
| ----------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `dprint` sets `textWrap: "never"`   | Every paragraph is unwrapped to one physical line. Do not hand-wrap prose.                                                                                                                                                                                               |
| `markdownlint` MD041, MD025         | One `#` heading, on the first line                                                                                                                                                                                                                                       |
| `markdownlint` MD001                | Heading levels increment by one                                                                                                                                                                                                                                          |
| `markdownlint` MD029                | Ordered lists renumber to `1.`, `2.`, `3.` — the only autofix                                                                                                                                                                                                            |
| `markdownlint` MD046                | Code blocks are fenced                                                                                                                                                                                                                                                   |
| `markdownlint` MD043                | Each fixed shape has one array in `.markdownlint/`, applied by its own `md-*` hook entry. Never set MD043 in the project config: it is merged over the shape and switches it off silently. Both faults are silent in hook output, so `repo_contracts::shape` gates them. |
| `relative-links`                    | A relative link must resolve to a real file, and a fragment to a real heading                                                                                                                                                                                            |
| pygrep link guards                  | Relative links must be explicit: `./name.md`, `../dir/name.md`, or `dir/name.md`. A bare `name.md` target is rejected.                                                                                                                                                   |
| `repo_contracts::emphasis`          | Decorative bold and italics are rejected outside fenced and inline code, over every document in the tree                                                                                                                                                                 |
| `repo_contracts::adrs`, `plan_zone` | ADR and plan-zone rules are checked over their complete zones, at push                                                                                                                                                                                                   |

Implemented slices may name exact nextest IDs because the pre-push resolver hook rejects duplicate and unresolved names.

## Further reading

- [`cargo-nextest` configuration](https://nexte.st/docs/configuration/reference/)
- [Testing — Command Line Applications in Rust](https://rust-cli.github.io/book/tutorial/testing.html)
- [`assert_cmd`](https://docs.rs/assert_cmd/), [`insta`](https://insta.rs/docs/), [`trycmd`](https://docs.rs/trycmd/)
