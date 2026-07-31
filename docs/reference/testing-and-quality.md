# Testing and quality

Test tools, test lanes, the gate map, and which contract each mandatory test locks down. The approach and its rationale are in [the testing strategy](../explanation/testing-strategy.md).

This describes normative design. The crate is pre-implementation.

## Test kinds

| Kind        | Location                                  | `nextest` filter                                      | What it tests                                                         |
| ----------- | ----------------------------------------- | ----------------------------------------------------- | --------------------------------------------------------------------- |
| Unit        | Inline `#[cfg(test)] mod tests` in `src/` | `kind(bin)`; `kind(lib)` once a library target exists | One function or module in isolation                                   |
| Integration | `tests/*.rs`                              | `kind(test)`                                          | The seam between components, against a stub child or a temporary tree |
| End-to-end  | Continuous integration only               | —                                                     | The whole product against the real `claude`                           |
| Doc tests   | `///` examples                            | Not run by `nextest`                                  | Requires a library target; none exists                                |

The defining property of an integration test is that it tests **an interaction**, not that it touches something real. A test running the compiled binary against a recording stub is an integration test.

End-to-end tests never run in a local hook. They are slow, they depend on host state, and they need credentials. A contributor must be able to run the whole local suite with nothing installed but the toolchain.

## Lanes

| Lane                   | Profile      | Runs                     | Budget               |
| ---------------------- | ------------ | ------------------------ | -------------------- |
| `pre-commit`           | `pre-commit` | Unit only                | Under one second     |
| `pre-push`             | `pre-push`   | Unit and integration     | Single-digit seconds |
| Continuous integration | `ci`         | Everything, with retries | Minutes              |

The unit lane is wired to **both** the commit and push hooks. It is nearly free, and running it again at push catches work that reached a commit through a rebase or an amend that bypassed the hook.

**The profile foot-gun:** a non-default `nextest` profile is inert unless the invoking command passes `--profile <name>`. A hook or recipe that omits it silently runs the default profile — every test, with the wrong settings — and the lane's filter never applies. Every invocation names its profile explicitly.

The time budget is one number: **the commit hook stays under one second of test time.** A commit hook slow enough to notice is a commit hook people bypass.

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

## Hermetic fixtures

Every test touching the environment or the filesystem must satisfy all of these:

| Requirement          | Rule                                                                                           |
| -------------------- | ---------------------------------------------------------------------------------------------- |
| Temporary directory  | Fresh per test, removed after. Never shared.                                                   |
| Child environment    | **Cleared, then explicitly populated.** Never inherited and patched.                           |
| Base directories     | Every `XDG_*` variable points inside the temporary directory                                   |
| Network              | None                                                                                           |
| Clock                | Injected where a timestamp is observable                                                       |
| Test-process globals | Never mutated. No environment mutation in the test process; no changing the working directory. |

The last row is the one that produces the worst bugs. Both the environment and the working directory are shared across a parallel test runner, so mutating either corrupts unrelated tests roughly one run in twenty — a failure rate that trains people to re-run rather than read.

## The recording stub

Real-process tests use a stub binary placed on the search path ahead of anything else. It records the arguments, environment, and working directory it received, and exits with whatever status the test requires — including death by a chosen signal.

The stub is what makes passthrough assertions mechanical: not "the command looked right" but "the child received exactly these arguments, in this order, with these bytes."

## Mandatory tests

Each of these locks down a contract that is otherwise decorative:

| Test                       | Locks                                                                           | Owning document                               |
| -------------------------- | ------------------------------------------------------------------------------- | --------------------------------------------- |
| Golden argv table          | Byte- and order-preserving passthrough, including empty and non-UTF-8 arguments | [CLI surface](./cli-surface.md)               |
| Exit-code matrix           | Every error variant maps to its documented code, no catch-all                   | [Exit codes](./exit-codes.md)                 |
| Child exit fidelity        | A stub exiting with N produces N                                                | [Exit codes](./exit-codes.md)                 |
| Child signal fidelity      | A signal-killed stub produces signal death, or the documented fallback          | [Exit codes](./exit-codes.md)                 |
| `--` sentinel              | A wrapper flag after `--` reaches the child uninterpreted                       | [CLI surface](./cli-surface.md)               |
| Recursion guard, marker    | The marker variable stops re-entry                                              | [Process runtime](./process-runtime.md)       |
| Recursion guard, self-path | The canonical self-check stops re-entry                                         | [Process runtime](./process-runtime.md)       |
| Environment isolation      | The stub sees the injected config directory and no internal variables           | [Process runtime](./process-runtime.md)       |
| Symlink rejection          | A session path that is a symlink is refused                                     | [XDG storage](./xdg-storage.md)               |
| Mode enforcement           | An over-permissive directory is corrected or refused                            | [XDG storage](./xdg-storage.md)               |
| Unknown configuration key  | A typo is rejected, naming the key and file                                     | [Configuration](./configuration.md)           |
| Merge determinism          | The same pieces produce byte-identical output                                   | [Configuration](./configuration.md)           |
| Freshness on piece change  | Editing a piece without the manifest triggers regeneration                      | [Configuration](./configuration.md)           |
| Example round-trip         | Every generated example parses through the real loader                          | [Configuration](./configuration.md)           |
| Undocumented field         | A public config field without a description fails generation                    | [Configuration](./configuration.md)           |
| Check-id coverage          | Every catalog id maps to an `err.kind` that exists                              | [Logging and output](./logging-and-output.md) |
| Help snapshot              | Generated help does not change unnoticed                                        | [CLI surface](./cli-surface.md)               |

Three of these have teeth beyond their own assertion. The exit-code matrix, written exhaustively over a closed enum, means adding an error variant without a code **fails the build**. The example round-trip is what stops a generated example from being a plausible-looking file the program itself would reject — an example that does not parse is worse than none, because the user trusts it. The undocumented-field test enforces the hard failure [ADR-0013](../decisions/ADR-0013-generate-config-examples-from-types.md) rests on: without it, the generator degrades quietly into emitting bare keys.

## The gate

`pre-commit run --all-files` is the single local command that reproduces the project's verdict. Hooks are the source of truth; task-runner gate recipes delegate to them, while inner-loop recipes stay raw `cargo`.

Run it inside the devShell. Several hooks take their binary from the shell rather than building one, so outside it they fail at exec rather than reporting on content ([ADR-0040](../decisions/ADR-0040-provision-hook-binaries-from-the-devshell.md)).

| Hook                                   | Stage        | Enforces                                       |
| -------------------------------------- | ------------ | ---------------------------------------------- |
| `cargo fmt`                            | commit       | Canonical formatting                           |
| `clippy` auto-fix, then gate           | commit       | Lint clean, warnings as errors                 |
| `cargo nextest` (`pre-commit` profile) | commit, push | Unit tests                                     |
| `cargo nextest` (`pre-push` profile)   | push         | Integration tests                              |
| `cargo test --doc`                     | push         | Doctests, guarded on a library target existing |
| `taplo`                                | commit       | TOML formatting                                |
| `typos`                                | commit       | Spelling                                       |
| `ripsecrets`                           | commit       | Fast secret scan                               |
| `gitleaks`                             | push         | Full secret scan                               |
| `cargo audit`                          | push         | Advisories                                     |
| `cargo deny`                           | push         | Advisories, bans, sources, licences            |
| `cargo machete`                        | push         | Unused dependencies                            |
| `cargo xtask gen-config`               | commit       | Generated examples match the config types      |
| `dprint`                               | commit       | Markdown and JSON formatting                   |
| `markdownlint-cli2`                    | commit       | Markdown structure and link integrity          |
| `shellcheck`, `shfmt`                  | commit       | Shell scripts                                  |
| `nixfmt`, `statix`, `deadnix`          | commit       | Nix sources                                    |
| `committed`                            | commit-msg   | Conventional Commits                           |

Fast, autofixing checks run at commit; slow and network-dependent ones at push. Do not bypass a hook. A hook that is wrong should be fixed in its configuration.

## Boundary lints

Two architectural rules are structural and are enforced by grep rather than by the compiler:

**Dependency direction.** `domain/` imports nothing from `adapters/` or `services/`. A violation means pure code has acquired an I/O dependency, and the type system will not catch it.

**Output ownership.** No print macro appears in `src/` outside the output module and the entry point. See [logging and output](./logging-and-output.md).

**Tooling isolation.** Nothing under `src/` imports from `xtask`, and the wrapper's own manifest does not list a development-tooling crate. The dependency runs one way, and the whole reason `xtask` exists is that its dependencies stay out of the shipped binary; see [dependencies](./dependencies.md).

Scope the first two to `src/`, and be aware that doc comments and test code produce false positives — the output-ownership check must not fire on an example inside a `///` block.

## Markdown

Documentation passes the same gate as code.

| Constraint                        | Consequence for authoring                                                                                              |
| --------------------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| `dprint` sets `textWrap: "never"` | Every paragraph is unwrapped to one physical line. Do not hand-wrap prose.                                             |
| `markdownlint` MD041, MD025       | One `#` heading, on the first line                                                                                     |
| `markdownlint` MD001              | Heading levels increment by one                                                                                        |
| `markdownlint` MD029              | Ordered lists renumber to `1.`, `2.`, `3.` — the only autofix                                                          |
| `markdownlint` MD046              | Code blocks are fenced                                                                                                 |
| `relative-links`                  | A relative link must resolve to a real file, and a fragment to a real heading                                          |
| pygrep link guards                | Relative links must be explicit: `./name.md`, `../dir/name.md`, or `dir/name.md`. A bare `name.md` target is rejected. |

Expect the first hook run after authoring to produce a large mechanical diff. Accept it and re-run.

## Further reading

- [`cargo-nextest` configuration](https://nexte.st/docs/configuration/reference/)
- [Testing — Command Line Applications in Rust](https://rust-cli.github.io/book/tutorial/testing.html)
- [`assert_cmd`](https://docs.rs/assert_cmd/), [`insta`](https://insta.rs/docs/), [`trycmd`](https://docs.rs/trycmd/)
