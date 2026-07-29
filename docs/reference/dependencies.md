# Dependencies

Which crates this project has reviewed, how one enters the manifest, and which are ruled out.

This describes normative design. The crate is pre-implementation and currently has no dependencies.

## No version numbers here

This page names crates and never versions. Versions are resolved by `cargo add` and recorded in the lockfile, which is committed. A version written into prose is stale the week after it is written, and a reader who trusts it pins the project to the past.

## Reviewed candidates

These crates have been assessed as appropriate for this project. **Being on this list does not put a crate in the manifest.** A crate is added when a specific piece of work needs it, and not before — an unused dependency is compile time, audit surface, and supply-chain risk bought for nothing. A lint catches unused dependencies; see [testing and quality](./testing-and-quality.md).

### Command line

| Crate                           | Why                                                           | Skip if                                                                      |
| ------------------------------- | ------------------------------------------------------------- | ---------------------------------------------------------------------------- |
| `clap` (derive, env, wrap_help) | The ecosystem standard. Derive keeps parse-shape declarative. | Never — the wrapper needs a parser                                           |
| `clap_complete`                 | Shell completions generated from the same grammar as help     | Completions are dropped                                                      |
| `clap_mangen`                   | Man pages from the same grammar                               | Never — [ADR-0016](../decisions/0016-ship-man-pages.md) commits to man pages |

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

| Crate            | Why                                                                              | Skip if                          |
| ---------------- | -------------------------------------------------------------------------------- | -------------------------------- |
| `serde` (derive) | The ecosystem standard                                                           | Never                            |
| `serde_json`     | The child's settings are JSON; the merge engine operates on its value type       | Never                            |
| `toml`           | The wrapper's own configuration format                                           |                                  |
| `serde_yaml_ng`  | Profile manifests are YAML. The maintained successor to the deprecated original. | Manifests move to another format |

### Configuration and paths

| Crate                 | Why                                                                                  | Skip if                                                           |
| --------------------- | ------------------------------------------------------------------------------------ | ----------------------------------------------------------------- |
| `figment` (env, toml) | Layered configuration with the precedence model this project needs, and provenance   | The layering is hand-rolled, which is more code for less          |
| `directories`         | XDG resolution with the specification's defaults, including ignoring relative values | An alternative XDG crate is preferred; both are acceptable        |
| `camino` (serde1)     | UTF-8 paths, so path handling in the config layer avoids lossy conversions           | Only where paths are known-UTF-8. Boundary paths stay OS strings. |

### Process and system

| Crate         | Why                                                                                           | Skip if                                                      |
| ------------- | --------------------------------------------------------------------------------------------- | ------------------------------------------------------------ |
| `rustix`      | Safe, direct system calls — ownership checks, process identity — without a raw `unsafe` block |                                                              |
| `signal-hook` | The widely-used signal handling crate; async-signal-safe registration                         |                                                              |
| `libc`        | Only where `rustix` has no equivalent                                                         | `rustix` covers the need, which it usually does              |
| `which`       | `PATH` search for the child binary                                                            | The search is hand-rolled, which is easy to get subtly wrong |
| `tempfile`    | Atomic write-then-rename, and hermetic test directories                                       | Never                                                        |

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

These are dependencies of the `xtask` workspace member ([ADR-0014](../decisions/0014-xtask-workspace-for-dev-tooling.md)) and **never enter the shipped binary's dependency graph**. That separation is the reason the generator lives in `xtask` at all, so adding one of these to the wrapper's own manifest defeats the point.

| Crate                   | Why                                                                              |
| ----------------------- | -------------------------------------------------------------------------------- |
| `schemars`              | JSON Schema derived from the configuration types; field docs become descriptions |
| `toml_edit`             | Renders the annotated example with comments, which a plain serializer cannot     |
| `claude-session` (path) | The library target, for the configuration types the generator reflects over      |

## Deferred

Reviewed, not needed yet. Named here so the decision is not re-made from scratch:

| Crate                                      | Unlocked by                                 |
| ------------------------------------------ | ------------------------------------------- |
| `serde_yaml_ng`                            | The first profile manifest                  |
| `rustix`, `signal-hook`                    | The first real spawn with signal forwarding |
| `which`                                    | The child resolution ladder                 |
| `tempfile`                                 | The first atomic write or hermetic test     |
| `clap_complete`, `clap_mangen`             | The completions and man-page work           |
| `schemars`, `toml_edit`                    | The `xtask` example generator               |
| `proptest`, `cargo-mutants`, `cargo-bloat` | The advanced test tier                      |

## Ruled out

| Crate              | Instead                     | Why                                                                              |
| ------------------ | --------------------------- | -------------------------------------------------------------------------------- |
| `dirs`             | `directories`               | Less maintained, and weaker about the specification's edge cases                 |
| `chrono`           | `time`                      | Historically larger audit surface; `time` covers this project's needs            |
| `lazy_static`      | `LazyLock`                  | Superseded by the standard library                                               |
| `serde_yaml`       | `serde_yaml_ng`             | Deprecated and unmaintained                                                      |
| `env_logger`       | `tracing-subscriber`        | This project uses `tracing`, and mixing facades gives two configuration surfaces |
| `structopt`        | `clap` derive               | Merged into `clap`                                                               |
| `failure`          | `thiserror` and `anyhow`    | Long deprecated                                                                  |
| `openssl` (direct) | The platform's TLS, or none | An unnecessary C build dependency for a program that makes no network calls      |

## Adding a dependency

**Always with `cargo add`.** Never by hand-editing the dependency table, and never by writing a version string:

```bash
cargo add clap --features derive,env,wrap_help
```

The tool resolves the graph, picks a compatible version, and updates the lockfile in one step. Hand-editing skips resolution, so the manifest and the lockfile disagree until something else fixes them, and hand-written versions are routinely stale or over-tight on the day they are written.

**The exception procedure.** Two cases justify deviating: a known-broken latest release, and a deliberate exact pin. Either requires a comment at the dependency saying which case it is and when to revisit — and if the reason is architectural rather than temporary, a decision record.

**Before adding anything not on this page**, check that it is maintained, that its licence is on the allow-list, that its transitive tree is proportionate to the problem, and that the standard library does not already solve it. Record the assessment in the change that adds it.

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
