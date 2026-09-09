# claude-session

[![OpenSSF Scorecard](https://api.scorecard.dev/projects/github.com/gubasso/claude-session/badge)](https://scorecard.dev/viewer/?uri=github.com/gubasso/claude-session)

A Rust CLI that wraps the `claude` command with session-oriented conveniences.

> The wrapper forwards to `claude` natively and owns `help`, `version`, `doctor`, the `account` and `session` namespaces, `completion`, `man`, `profile`, and `config`. Everything else described in [docs](./docs/README.md) is design ahead of the code.

## Usage

Anything the wrapper does not own reaches `claude` unchanged — same arguments, same standard streams, same exit status. There is one process by the time the child runs, so signals and terminal behaviour are the child's too.

```bash
claude-session                       # exactly `claude`
claude-session -p "hello" --verbose  # forwarded verbatim
claude-session -- --version          # `--` forces every later argument to the child
```

Two of the wrapper's surfaces compose their own output with the child's:

```bash
claude-session version   # the wrapper's version, then the resolved child's
claude-session help      # the wrapper's help, then `claude --help`
```

## Authentication

An account is one isolated `claude` configuration directory, and every launch is bound to exactly one. An account is in one of two authentication modes at any time. What follows is the path through them; [accounts](./docs/reference/accounts.md) is the contract.

### Native login

`account login <name>` hands the browser flow to `claude` itself. The wrapper resolves the account directory, points the child's own configuration there, and runs the child's `auth login` inside it. It does not implement the flow and does not read its result.

```console
$ claude-session account login work
Opening browser to sign in…
If the browser didn't open, visit: https://claude.com/cai/oauth/authorize?code=true&client_id=...
Paste code here if prompted > Login successful.
account: work
mode: login
path: /home/you/.local/state/claude-session/accounts/work
recorded_at: 2026-08-13T14:25:01Z
```

Everything through `Login successful.` is the child talking to you. The labelled lines under it are the wrapper's report, and that split is the design: the child owns how a credential is obtained, the wrapper owns where it lands. The same login is reachable without the verb, as `claude-session --account work -- auth login`.

### Long-lived token

`account login <name> --token` puts the account in token mode, where the wrapper stores one long-lived subscription token and injects it into every launch. Use it where no browser can open, or where authentication must outlive an interactive session.

Interactively, the wrapper runs the child's `setup-token` with inherited streams and then asks for the result back:

```console
$ claude-session account login work --token
 ✓ Long-lived authentication token created successfully!

 Your OAuth token (valid for 1 year):

 sk-ant-oat01-...

 Store this token securely. You won't be able to see it again.

Paste the token, then press Enter (it is not echoed):
account: work
mode: token
path: /home/you/.local/state/claude-session/accounts/work
recorded_at: 2026-08-13T14:28:09Z
fingerprint: 76721470
estimated_expiry: 2027-08-13T14:28:09Z
```

The paste step is deliberate. The token reaches the wrapper only because you hand it over, so the wrapper never parses the child's presentation, which is not a contract it can depend on. Echo is off while you type, a multi-line paste is refused whole rather than half-stored, and the token never passes through argv or the environment ([ADR-0027](./docs/decisions/ADR-0027-ingest-secrets-only-from-stdin-or-a-terminal.md)).

Two fields are token mode only. `fingerprint` is `sha256[..8]` of the token, which lets `account status` say which token an account holds without printing one. `estimated_expiry` is `recorded_at` plus 365 days, and it is always an estimate: the wrapper does not read the token, and the child gives no expiry warning for an injected one, which simply stops working. Pasting a token minted earlier, `--minted-at` corrects the clock both are derived from.

### Token entry without a terminal

`--stdin` reads the token from standard input, as one line, without prompting. It skips `setup-token` entirely and needs no controlling terminal, which is what makes it the automation path: interactive entry requires one and exits `Unavailable` (69) where there is none.

```bash
pass show anthropic/ci-token | claude-session account login ci --token --stdin
claude-session account login ci --token --stdin < token.txt
```

Standard input is the only door. A positional argument would land in shell history, an environment variable could not be told apart from an inherited ambient credential, and a file flag would duplicate what redirection already does.

### Living with an account

`account login` is idempotent. The first success creates the account, a later success replaces its mode, and a failed first login removes the incomplete account. So running `--token` against an account that logged in natively switches it to token mode: the saved login stays on disk, the injected token shadows it on every launch, and `account status` reports both.

```bash
claude-session account list                  # every account, its mode, and which is selected
claude-session account status work           # mode, health, provenance, and what shadows what
claude-session --account work --profile dev  # launch bound to that account
claude-session account remove work           # local state only; prompts unless --yes
```

Removal is local. It cannot revoke a token upstream, and the report says so.

## Profiles

Profiles compose user-authored settings pieces into one child settings document, selected per launch:

```bash
mkdir -p ~/.config/claude-session/settings ~/.config/claude-session/profiles
echo '{"model":"sonnet"}' > ~/.config/claude-session/settings/base.json
printf 'layers:\n  - base\n' > ~/.config/claude-session/profiles/dev.yaml

claude-session profile               # the profiles `--profile` can select
claude-session --profile dev         # launch with the composed settings
claude-session --profile dev config  # what resolved, from where, and into what
```

Composition is ordered and explainable. Later pieces win, and a key that needs appending rather than replacing says so:

```bash
echo '{"permissions":{"allow":["Bash(git status:*)"]}}' \
  > ~/.config/claude-session/settings/git.json
cat > ~/.config/claude-session/profiles/dev.yaml <<'YAML'
layers:
  - base
  - git
array_strategies:
  "/permissions/allow": { strategy: concat }
YAML
```

The provenance sidecar beside each composed entry records, per key, which piece won and which it overrode.

The keys above are illustrative. A piece is written in the child's own settings format, and the wrapper models none of it: it validates that the composition is well defined, then forwards every key you wrote, whatever it is called.

The composed document reaches the child as an additional native settings layer, so a `--settings` of your own still replaces it and the working directory's own settings still load beneath it. [Milestones](./docs/plan/milestones.md) carries the order the rest lands in.

## Sessions

Every running claude gets its own child state directory, so two agents never interleave their prompt history or their child configuration, while the account's login and projects tree stay shared. A launch records which process the directory belongs to, and that record is what makes cleanup honest later:

```bash
claude-session session list   # every session directory, and whether its agent is still running
claude-session session clean  # remove the provably dead ones; prompts unless --yes
```

A launch removes the account's sessions whose agent has exited; `session clean` removes everything it cannot prove is still running, including what it cannot decide about at all. The verdicts and their reasoning live in [sessions](./docs/reference/sessions.md).

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

## Install

The crate is on crates.io. [ADR-0022](./docs/decisions/ADR-0022-cut-the-first-release-when-passthrough-works.md) held the first release back until the wrapper forwarded to `claude`.

```bash
cargo install claude-session
```

Running it needs `claude` on `PATH`. The wrapper resolves it, hands it the argument vector, and becomes it.

## Build from source

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

Report a vulnerability privately instead of opening an issue. [The security policy](./SECURITY.md) has the channel.
