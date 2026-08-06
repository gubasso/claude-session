# Architecture

This page gives the mental model of the `claude-session` crate: the shape of the code, the vocabulary the rest of the documentation uses, the invocation lifecycle, and what each module may and may not do. It is the map you should hold in your head before writing a line of code.

It does not carry exact values. Naming rules, visibility defaults, and the error type stack are in [coding conventions](../reference/coding-conventions.md); the crate list is in [dependencies](../reference/dependencies.md); the wrapper's own grammar is in [the CLI surface](../reference/cli-surface.md). The native-passthrough foundation follows this architecture; later-slice modules remain specified rather than shipped.

## A human-facing tool

Before the shapes, the premise everything else rests on: `claude-session` is human-facing. Its primary consumer is a developer at a terminal, and its main mode is handing that terminal to a child which is itself interactive ([ADR-0017](../decisions/ADR-0017-declare-a-human-facing-cli.md)).

That single choice settles three things you would otherwise have to guess at. Machine-readable output is opt-in through `--json` rather than the default. The terminal-output module is `ui/` — shaped for human rendering, not a protocol boundary. Diagnostics mirror to standard error by default, because a person should not have to enable seeing them.

It is a design-time category, not a runtime `isatty()` check. Terminal detection changes colour and progress rendering and nothing else; it never changes which format a command emits. A program whose output reshapes itself when piped cannot be scripted against.

## The two shapes

The single most useful distinction in this codebase is between parse-shape and runtime-shape.

Parse-shape is what the command line looked like. It is a `clap` derive struct: every field optional or defaulted, everything a string or a path or a count, nothing validated beyond what the parser can check syntactically. Parse-shape types live in `cli/` and nowhere else.

Runtime-shape is what the program actually needs. Paths are resolved and absolute, identifiers are validated newtypes, precedence between the flag and the environment variable and the config file has already been settled, and illegal states are unrepresentable. Runtime-shape types live in `domain/`, and the conversion from parse-shape to runtime-shape happens once, at the boundary, in `commands/`.

Keeping the two apart is what stops `Option<String>` from leaking three layers deep. The rule of thumb: if a type still remembers that a value came from a flag rather than from config, it is parse-shape and belongs at the edge.

## The invocation lifecycle

Every run of the binary follows the same five-step spine:

1. Parse — split argv into what the wrapper owns and what belongs to the child, then parse the wrapper's part. Nothing here touches the filesystem or the network.
2. Resolve — resolve XDG paths, install logging, load layered configuration, and build the `AppContext`. Paths precede logging because the subscriber writes into the state namespace, and both precede the context because a configuration failure has to be logged.
3. Prepare — for a passthrough invocation, resolve the account auth mode, validate its config and token state, resolve the profile, and ensure its composed settings entry exists.
4. Supervise — resolve the child binary, spawn it, forward signals, wait.
5. Post-flight — update last-used state, finalize logs, then map the outcome to an exit code.

Step 5 is why this program spawns and waits rather than `exec`-ing; see [the wrapper model](./wrapper-model.md).

`main.rs` is that spine and nothing more, and it holds no logic of its own: it connects to process globals, installs logging, calls the fallible program, and converts the outcome into a status. The fallible program is a separate function that receives what it needs as parameters and reads no global, which is what makes `?` available below the entry point and the argument grammar exercisable without a process. The order of the conversion — report, flush, then exit — is fixed by [ADR-0080](../decisions/ADR-0080-order-the-boundary-as-report-flush-exit.md). It stays at or under 120 lines. When `main.rs` grows, the growth belongs in a command handler or a service, not here.

## Module roles

The shipped crate has a flat module tree. Each module has one job and one explicit prohibition. (The `xtask` member is development tooling and is not part of this map; see [below](#one-shipped-crate-plus-xtask).)

`cli/` holds `clap` derive structs and nothing else — the root parser, the shared global arguments, and one file per wrapper verb. It does not contain logic, I/O, or any decision about what a flag means. A `cli/` file that calls a function outside `clap` is misplaced.

`commands/` holds one handler per wrapper verb, each a free function taking the context and the verb's parsed arguments and returning the crate's result type. A handler orchestrates: it converts parse-shape to runtime-shape, calls services and adapters, and hands results to the output writer. It does not contain reusable business logic — the moment two handlers want the same routine, that routine moves to `services/`.

`domain/` holds pure types and their invariants: validated newtypes, the child-invocation model, the session identity. It does not perform I/O. No filesystem, no network, no process spawning, no clock. Deriving serialization on a domain type is fine; calling a deserializer is not — that is a boundary concern.

`services/` holds orchestration that more than one caller needs: the profile resolver, the account-mode gate, the settings composer. A service takes its dependencies as trait parameters so a test can substitute a fake. It is optional in principle and populated in practice, but a routine only earns a place here once it has a second caller or a real invariant to protect.

`adapters/` is the only place that touches the outside world. Each adapter defines a trait — the port — plus a default implementation named for the system it reaches. A port carries what its callers need rather than what its implementation happens to hold: a method returning a type only `std::fs` can construct cannot be faked, which defeats the point of having a port. The filesystem, the process spawner, and the environment snapshot all arrive through this module. This is the seam that makes a process-spawning wrapper testable at all; see [the testing strategy](./testing-strategy.md).

`config/` owns the layered configuration loader and produces one immutable resolved value. `context.rs` owns the `AppContext`. `error.rs` owns the error stack and the exit-code mapping. `logging.rs` installs exactly one subscriber. `ui/` owns every byte written to a terminal. `util/` holds genuinely generic helpers and is the module most likely to become a junk drawer — resist it.

The dependency direction runs one way: `main` → `commands` → `services` → `adapters` and `domain`. `domain` depends on nothing in the crate. An import that runs backwards up that chain is a design error, and a lint catches it; see [testing and quality](../reference/testing-and-quality.md).

## AppContext

One `AppContext` is built at the entry point, after logging is installed, and then passed by shared reference to everything downstream. It carries the resolved configuration, the resolved XDG paths, the output format, the output writer, and the concrete adapters. Verbosity is not on it: the ladder governs the diagnostic mirror and nothing else, so it is consumed once when the subscriber is installed and never read again.

It is immutable after construction. That is the whole point: a context that can be mutated mid-run is a global variable with extra steps, and it makes every function's behaviour depend on call order. Values that genuinely change during a run — the child's process id, for instance — live behind their own synchronized cell, not in the context's shape.

There are no other globals. No `static mut`, no lazily-initialized singleton holding configuration, no ambient `std::env::var` read from deep inside a service. Anything a function needs, it receives.

## Adding a verb: the four-edit rule

Adding a wrapper verb touches exactly four files, in this order:

1. `cli/<verb>.rs` — a new `clap` arguments struct for the verb.
2. `cli.rs` — a new variant on the commands enum, referencing that struct.
3. `commands/<verb>.rs` — the handler, a free function over the context and the arguments.
4. `commands/dispatch.rs` — a new match arm routing the variant to the handler.

If a change needs a fifth file, it is not just a new verb — it is a new verb plus something else, and the something else should be a separate, reviewable step. If it needs fewer than four, a layer is being skipped and parse-shape is about to leak.

The symmetry is deliberate: one file per verb on the parse side, one file per verb on the runtime side. You can find either half from the other by name alone.

## Help text

`--help` is generated by the parser. It is never hand-maintained, because a hand-written flag table drifts from the parser the first time someone adds a flag and forgets the table.

Authored prose that the parser cannot generate — worked examples, the passthrough explanation, a pointer at the docs — lives in a separate text file under `ui/` and is included into the parser's long-help at compile time. That way the prose is version-controlled next to the code, and the flag list stays generated.

## One shipped crate, plus `xtask`

The design intent was one binary crate and no workspace, with migration deferred until a concrete trigger fired:

- A second binary genuinely needs to share code with the first.
- A subsystem becomes worth publishing on its own.
- `cargo check` gets slow enough to hurt the inner loop.
- The crate approaches roughly eight thousand lines.

The first trigger has fired, for exactly the reason it was written down. The configuration-example generator ([configuration](../reference/configuration.md)) has to reflect over the wrapper's own configuration types, and a binary-only crate cannot export them. The repository therefore becomes a two-member workspace: the crate gains a library target exposing what the tooling needs, and `xtask/` is a second binary depending on it by path ([ADR-0014](../decisions/ADR-0014-xtask-workspace-for-dev-tooling.md)).

The conversion has not happened. `Cargo.toml` declares no workspace and there is no `xtask/` on disk; ADR-0014 is `Accepted`, which means the choice governs and not that it is built. [ADR-0078](../decisions/ADR-0078-adopt-the-seven-state-decision-lifecycle.md) defines authority, and [the template](../decisions/template.md) defines status syntax. The slice that writes the generator performs the conversion, and everything below describes the shape it produces.

`xtask` is development tooling and never shipped surface. It is invoked as `cargo xtask <chore>`, it is not installed, and it is not on the CLI grammar in [the CLI surface](../reference/cli-surface.md). Its dependencies — the schema and rendering crates — live in its own manifest and never enter the binary a user installs. Nothing in `xtask/` may be imported by the wrapper; the dependency runs one way, as everywhere else here.

The library target exists to serve that tooling, not to become a public API. Only what `xtask` needs is exported; a `lib.rs` that grows into a re-export of every private module is the dead weight this section was originally guarding against.

The three remaining triggers still gate any further split. Splitting later is mechanical; splitting early is a permanent tax.

## Further reading

- [The Rust CLI Book](https://rust-cli.github.io/book/)
- [Command Line Interface Guidelines](https://clig.dev/)
- [Parse, don't validate](https://lexi-lambda.github.io/blog/2019/11/05/parse-don-t-validate/)
- [The newtype pattern](https://rust-unofficial.github.io/patterns/patterns/behavioural/newtype.html)
