# Dependencies

Which crates this project has reviewed, how one enters the manifest, and which are ruled out.

This describes normative design. The crate is pre-implementation and currently has no dependencies.

## No version numbers here

This page names crates and never versions. Versions are resolved by `cargo add` and recorded in the lockfile, which is committed. A version written into prose is stale the week after it is written, and a reader who trusts it pins the project to the past.

## Reviewed candidates

These crates have been assessed as appropriate for this project. **Being on this list does not put a crate in the manifest.** A crate is added when a specific piece of work needs it, and not before — an unused dependency is compile time, audit surface, and supply-chain risk bought for nothing. A lint catches unused dependencies; see [testing and quality](./testing-and-quality.md).

### Command line

| Crate                           | Why                                                           | Skip if                                                                          |
| ------------------------------- | ------------------------------------------------------------- | -------------------------------------------------------------------------------- |
| `clap` (derive, env, wrap_help) | The ecosystem standard. Derive keeps parse-shape declarative. | Never — the wrapper needs a parser                                               |
| `clap_complete`                 | Shell completions generated from the same grammar as help     | Completions are dropped                                                          |
| `clap_mangen`                   | Man pages from the same grammar                               | Never — [ADR-0016](../decisions/ADR-0016-ship-man-pages.md) commits to man pages |

Note that `clap` alone cannot express this wrapper's passthrough; see [the CLI surface](./cli-surface.md) for the pre-split contract.

### Errors

| Crate       | Why                                                        | Skip if                                                                                   |
| ----------- | ---------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| `thiserror` | Derives typed error enums without hand-written boilerplate | Never — the exit-code matrix needs typed errors                                           |
| `anyhow`    | Ergonomic error reporting at the outermost boundary        | Its use is confined to the entry point; see [coding conventions](./coding-conventions.md) |

### Diagnostics

| Crate                                  | Why                                            | Skip if                                   |
| -------------------------------------- | ---------------------------------------------- | ----------------------------------------- |
| `tracing`                              | Structured, leveled instrumentation with spans | Never                                     |
| `tracing-subscriber` (env-filter, fmt) | Subscriber and `RUST_LOG` filtering            | Never                                     |
| `tracing-appender`                     | Non-blocking file sink with rotation           | The file sink is dropped, which it is not |

### Serialization

| Crate            | Why                                                                        | Skip if                         |
| ---------------- | -------------------------------------------------------------------------- | ------------------------------- |
| `serde` (derive) | The ecosystem standard                                                     | Never                           |
| `serde_json`     | The child's settings are JSON; the merge engine operates on its value type | Never                           |
| `toml`           | The wrapper's own configuration format                                     |                                 |
| `serde_yaml_ng`  | Profiles are YAML. The maintained successor to the deprecated original.    | Profiles move to another format |

### Configuration and paths

| Crate                 | Why                                                                                  | Skip if                                                           |
| --------------------- | ------------------------------------------------------------------------------------ | ----------------------------------------------------------------- |
| `figment` (env, toml) | Layered configuration with the precedence model this project needs, and provenance   | The layering is hand-rolled, which is more code for less          |
| `directories`         | XDG resolution with the specification's defaults, including ignoring relative values | An alternative XDG crate is preferred; both are acceptable        |
| `camino` (serde1)     | UTF-8 paths, so path handling in the config layer avoids lossy conversions           | Only where paths are known-UTF-8. Boundary paths stay OS strings. |

### Process and system

| Crate         | Why                                                                                                                | Skip if                                                      |
| ------------- | ------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------ |
| `rustix`      | Safe, direct system calls — ownership checks, process identity — without a raw `unsafe` block                      |                                                              |
| `signal-hook` | The widely-used signal handling crate; async-signal-safe registration                                              |                                                              |
| `libc`        | Only where `rustix` has no equivalent                                                                              | `rustix` covers the need, which it usually does              |
| `which`       | `PATH` search for the child binary. It emulates `which(1)`, so the wrapper drops zero-length `PATH` entries itself | The search is hand-rolled, which is easy to get subtly wrong |
| `tempfile`    | Atomic write-then-rename, and hermetic test directories                                                            | Never                                                        |

### Asynchrony

| Crate                | Why                                     | Skip if                                                                                                                                                                   |
| -------------------- | --------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `tokio` (rt, macros) | Only if concurrency is genuinely needed | **Probably skip.** A wrapper that spawns one child and waits has no need for an async runtime. Add it only when a specific requirement demands it, and record the reason. |

### Testing

| Crate        | Why                                                      |
| ------------ | -------------------------------------------------------- |
| `assert_cmd` | Runs the compiled binary and asserts on its behaviour    |
| `predicates` | Composable assertions on output                          |
| `insta`      | Snapshot testing, for help output and structured reports |
| `assert_fs`  | Filesystem fixtures and assertions                       |
| `tempfile`   | Hermetic per-test directories                            |
| `proptest`   | Property tests, notably argv round-tripping              |

### Development tooling — `xtask` only

These are dependencies of the `xtask` workspace member ([ADR-0014](../decisions/ADR-0014-xtask-workspace-for-dev-tooling.md)) and **never enter the shipped binary's dependency graph**. That separation is the reason the generator lives in `xtask` at all, so adding one of these to the wrapper's own manifest defeats the point.

| Crate                   | Why                                                                              |
| ----------------------- | -------------------------------------------------------------------------------- |
| `schemars`              | JSON Schema derived from the configuration types; field docs become descriptions |
| `toml_edit`             | Renders the annotated example with comments, which a plain serializer cannot     |
| `claude-session` (path) | The library target, for the configuration types the generator reflects over      |

## Deferred

Reviewed, not needed yet. Named here so the decision is not re-made from scratch:

| Crate                                      | Unlocked by                                                   |
| ------------------------------------------ | ------------------------------------------------------------- |
| `serde_yaml_ng`                            | The first profile                                             |
| `rustix`                                   | The confirmation-prompt test harness, or the first real spawn |
| `signal-hook`                              | The first real spawn with signal forwarding                   |
| `which`                                    | The child resolution ladder                                   |
| `tempfile`                                 | The first atomic write or hermetic test                       |
| `clap_complete`, `clap_mangen`             | The completions and man-page work                             |
| `schemars`, `toml_edit`                    | The `xtask` example generator                                 |
| `proptest`, `cargo-mutants`, `cargo-bloat` | The advanced test tier                                        |

## Ruled out

| Crate                 | Instead                                          | Why                                                                                                                                                                                                                                                             |
| --------------------- | ------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `dirs`                | `directories`                                    | Less maintained, and weaker about the specification's edge cases                                                                                                                                                                                                |
| `chrono`              | `time`                                           | Historically larger audit surface; `time` covers this project's needs                                                                                                                                                                                           |
| `lazy_static`         | `LazyLock`                                       | Superseded by the standard library                                                                                                                                                                                                                              |
| `serde_yaml`          | `serde_yaml_ng`                                  | Deprecated and unmaintained                                                                                                                                                                                                                                     |
| `env_logger`          | `tracing-subscriber`                             | This project uses `tracing`, and mixing facades gives two configuration surfaces                                                                                                                                                                                |
| `structopt`           | `clap` derive                                    | Merged into `clap`                                                                                                                                                                                                                                              |
| `failure`             | `thiserror` and `anyhow`                         | Long deprecated                                                                                                                                                                                                                                                 |
| `sysexits`            | One hand-rolled enum                             | It cannot express the codes this wrapper owns outside the convention — `1`, `126`, `127`, a child status in `0..=255` — so adopting it would split the code set across two owners ([ADR-0035](../decisions/ADR-0035-convert-the-typed-error-to-a-code-once.md)) |
| `openssl` (direct)    | The platform's TLS, or none                      | An unnecessary C build dependency for a program that makes no network calls                                                                                                                                                                                     |
| Rust `keyring` family | Private file or out-of-process credential helper | Its headless-Linux keyutils backend is memory-only and cannot provide reboot persistence; [ADR-0029](../decisions/ADR-0029-use-a-credential-helper-process-boundary.md) keeps secure-store integration outside the process                                      |

Pseudo-terminal scraping of child token output is rejected architecturally by [ADR-0027](../decisions/ADR-0027-ingest-secrets-only-from-stdin-or-a-terminal.md). It authorizes no PTY or presentation-parser dependency; no unassessed crate is named.

## Adding a dependency

Follow this admission procedure in order:

1. Name the concrete requirement and the durable contract or round that needs it now. Unused or speculative additions fail.
2. Select the target graph explicitly: shipped wrapper, test/development only, or `xtask`. Development-tooling crates never enter the shipped graph.
3. Require the crate to appear in the reviewed table. If it is deferred, its named unlock condition must have occurred. A ruled-out crate requires a replacement or superseding ADR when its rejection is architectural.
4. Before adding an unlisted crate, update this page with its maintenance or successor status, licence compatibility, RustSec and advisory state, MSRV compatibility, minimal feature set, direct purpose, transitive size and duplication, and why the standard library or an existing crate is insufficient. A hard-to-back-out dependency or policy exception requires an ADR; a routine reviewed candidate does not.
5. Add it with `cargo add` against the correct package and dependency class, using `--dev` for test-only use, only required features, and disabled default features when the assessment says they are unnecessary. Let Cargo write the version and lockfile.
6. Use a known-broken-latest workaround or deliberate exact pin only with a manifest-adjacent reason, upstream issue or source, and revisit trigger. Use an ADR when the exception is architectural rather than temporary.
7. Verify `cargo metadata --locked`, the round's build and tests, `cargo deny check advisories bans sources licenses`, `cargo audit`, `cargo machete`, and the full pre-commit gate. Remove a dependency with `cargo rm`, then rerun the same checks.

`cargo add` is the only normal writer: never hand-edit a dependency table or write a version string. For example:

```bash
cargo add clap --features derive,env,wrap_help
```

The states are distinct: **reviewed** authorizes consideration, not installation; **deferred** names the trigger; the manifest records present use; the lockfile records resolution; **ruled out** records a rejected approach. They are not interchangeable statuses.

## Lockfile and supply chain

`Cargo.lock` is **committed**. This is a binary, not a library: reproducible builds are the point, and a lockfile is how a bug report from six months ago is reproducible.

| Gate                          | Enforces                                                                  |
| ----------------------------- | ------------------------------------------------------------------------- |
| `cargo deny check advisories` | No known-vulnerable dependency                                            |
| `cargo deny check bans`       | No wildcard version requirement; no banned crate; no gratuitous duplicate |
| `cargo deny check sources`    | Every crate comes from a known registry                                   |
| `cargo deny check licenses`   | Every licence is on the allow-list                                        |
| `cargo audit`                 | Independent advisory check                                                |
| `cargo machete`               | No declared-but-unused dependency                                         |

The project is dual-licensed MIT or Apache-2.0, and the allow-list is compatible with both.

## Further reading

- [The Cargo Book: specifying dependencies](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html)
- [`cargo-deny`](https://embarkstudios.github.io/cargo-deny/)
- [The RustSec advisory database](https://rustsec.org/)
