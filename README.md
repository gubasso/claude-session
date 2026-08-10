# claude-session

A Rust CLI that wraps the `claude` command with session-oriented conveniences.

> Status: unreleased. The wrapper forwards to `claude` natively, and `help` and `version` are the only surfaces it owns. Nothing else described in [docs](./docs/README.md) is shipped yet.

## Usage

Anything the wrapper does not own reaches `claude` unchanged — same arguments, same standard streams, same exit status. There is one process by the time the child runs, so signals and terminal behaviour are the child's too.

```bash
claude-session                       # exactly `claude`
claude-session -p "hello" --verbose  # forwarded verbatim
claude-session -- --version          # `--` forces every later argument to the child
```

Two surfaces belong to the wrapper, and each composes its own output with the child's:

```bash
claude-session version   # the wrapper's version, then the resolved child's
claude-session help      # the wrapper's help, then `claude --help`
```

The account and profile isolation the crate description promises is not shipped. [Milestones](./docs/plan/milestones.md) carries the order it lands in.

## Design contract

- Never break native `claude` passthrough. Anything the wrapper does not own is forwarded to `claude` unchanged, including arguments, stdin/stdout, and exit codes.
- XDG-compliant. Every file the wrapper writes goes to its [XDG base directory](./docs/reference/xdg-storage.md).
- Self-contained. At runtime the tool needs nothing but itself and the `claude` binary. Building it needs the pinned devShell.

These bullets summarize the direct owners linked from [the documentation index](./docs/README.md).

## Documentation

The engineering specifications live under [docs](./docs/README.md), organized by reader need. Current delivery order lives in [milestones](./docs/plan/milestones.md).

Common starting points:

- What is this and how is it built? → [architecture](./docs/explanation/architecture.md)
- What does it mean to wrap `claude`? → [the wrapper model](./docs/explanation/wrapper-model.md)
- How do I work on it? → [the development workflow](./docs/guides/development-workflow.md)

## Build from source

There is no published release. [ADR-0022](./docs/decisions/ADR-0022-cut-the-first-release-when-passthrough-works.md) holds the first crates.io version until the wrapper forwards to `claude`, which it now does; cutting `0.1.0` is a separate human decision, so a checkout is the only way to install today.

Running the result needs `claude` on `PATH`. The wrapper resolves it, hands it the argument vector, and becomes it.

```bash
# Clone the repository
git clone https://github.com/gubasso/claude-session.git
cd claude-session

# Build and install. Cargo owns the destination; the wrapper writes nothing there.
cargo install --path .
```

## Development shell

This project ships a Nix flake devShell with all tooling pinned.

```bash
# Interactive: allow direnv to load the shell on cd
direnv allow

# Ad hoc: enter the devShell directly
nix develop
```

## Tasks

Common tasks run through the project task runner:

```bash
just lint    # run linters and formatters
just test    # run the test suite
just build   # build the project
just hooks   # the full gate, both hook stages — this is the verdict
```

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this crate by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

## Contributing

Contributions are welcome. Please open an issue to discuss substantial changes before submitting a pull request, and run the full gate first — [the development workflow](./docs/guides/development-workflow.md) has the procedure.
