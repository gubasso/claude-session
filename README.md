# claude-session

A Rust CLI that wraps the `claude` command with session-oriented conveniences.

> **Status: pre-implementation.** The crate currently builds a placeholder binary. Nothing below describes shipped behaviour beyond the build and development workflow.

## Design contract

- **Never break native `claude` passthrough.** Anything the wrapper does not own is forwarded to `claude` unchanged, including arguments, stdin/stdout, and exit codes.
- **XDG-compliant.** Configuration, state, and cache live under the standard XDG base directories.
- **Self-contained.** The tool depends on nothing outside the repository and the `claude` binary itself.

These three bullets are a summary. The normative sources are [AGENTS.md](./AGENTS.md) for the contracts themselves and [docs/decisions/](./docs/decisions/) for the recorded decisions behind them.

## Documentation

The engineering specifications live under [docs/](./docs/README.md), organized by what a reader needs: decisions, explanation, reference, and guides. They describe design for code that has not been written yet.

Common starting points:

- What is this and how is it built? → [architecture](./docs/explanation/architecture.md)
- What does it mean to wrap `claude`? → [the wrapper model](./docs/explanation/wrapper-model.md)
- How do I work on it? → [the development workflow](./docs/guides/development-workflow.md)

## Install

```bash
# Clone the repository
git clone https://github.com/gubasso/claude-session.git
cd claude-session

# Build and install the binary into ~/.cargo/bin
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
```

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this crate by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

## Contributing

Contributions are welcome. Please open an issue to discuss substantial changes before submitting a pull request, and ensure `just lint` and `just test` pass.
