# Coding conventions

Lookup rules for code shape: naming, visibility, module form, the error type stack, and the panic policy. The reasoning behind the module layout is in [the architecture](../explanation/architecture.md).

These conventions govern the implemented native-passthrough foundation and every later slice.

## Naming

| Kind                         | Pattern                 | Example                          |
| ---------------------------- | ----------------------- | -------------------------------- |
| Parse-shape argument struct  | `<Verb>Args`            | `AccountArgs`                    |
| Runtime-shape request        | `<Verb>Request`         | `ComposeRequest`                 |
| Per-layer error enum         | `<Layer>Error`          | `DomainError`, `ConfigError`     |
| Validated newtype            | The concept, singular   | `AccountId`, `ProfileId`         |
| Adapter trait                | The role it plays       | `Spawner`, `Filesystem`, `Clock` |
| Command handler              | `run`                   | One per `commands/<verb>.rs`     |
| Constructor returning `Self` | `new`                   |                                  |
| Fallible constructor         | `try_new`, or `FromStr` |                                  |
| Conversion, borrowed, cheap  | `as_*`                  | `as_str`                         |
| Conversion, owned, cheap     | `into_*`                | `into_inner`                     |
| Conversion, expensive        | `to_*`                  | `to_args`                        |

Banned suffixes: `Manager`, `Helper`, `Util`, `Handler`, `Data`, `Info`, `Impl`. Each names a shape rather than a job, and a type that cannot be named after its job usually has more than one.

Modules are singular when they hold one concept, plural when they hold a collection of siblings: `context.rs`, but `commands/`.

Booleans read as assertions: `is_executable`, `has_credentials`. Never a negation in the name — `is_not_ready` produces `!is_not_ready` at the call site.

## Function placement

A function that takes a receiver is a method. That much is settled by ownership rather than taste, and it is the whole of what the API guidelines' `C-METHOD` governs. Everything else is receiver-free, and the choice between an associated function and a free function turns on the specificity of the input, not on which type comes out.

Associated when the function's one job is producing that type from another type the crate already trusts. `XdgPaths::resolve`, `Color::resolve`, and `SessionPaths::resolve` are the instances here: one output, one owner, one door.

Free when the input is the shape the outside world hands over — raw argv, an environment slice, a file — or when the function is a named stage of a pipeline its module owns. `argv::split`, `dispatch::classify`, `dispatch::dispatch`, `config::load::resolve`, `environment::value`, `logging::install`, and `ui::writer::report` are all this case. Hanging one on the type it returns inflates that type's dependencies to reach a call site that already reads correctly, and turns an inert data carrier into something that knows its own handlers.

The trap is treating "it constructs one type" as sufficient. `C-CONV-SPECIFIC` places a conversion on the most specific type involved, and `&[OsString]` is the least specific type in the crate; there is nothing on the input side to hang it on, and the rule supplies no pull toward the output side either. Rust's own boundary parsers land the same way: `rustc_parse::new_parser_from_file` builds a `Parser` as a free function, `serde_json::from_str` builds a caller-chosen `T`, and ripgrep splits the two cases precisely — `flags::parse` reads raw argv freely, while `HiArgs::from_low_args` converts one settled type into another as an associated function.

A receiver-free associated function also collects none of `C-METHOD`'s benefits. No autoref, no import saved, and with no library target there is no rustdoc audience to discover it. What is left is the module path, and `commands::dispatch::classify` names the stage where `Invocation::classify` would restate a noun its module already carries.

When a function does belong on a type, `C-CTOR` fixes the name: `from_<source>` for a conversion constructor, or a domain verb in the shape of `File::open`. `try_new` is for a general fallible constructor and nothing else.

## Visibility

`pub(crate)` is the default for everything. There is no library target, so `pub` on an item that nothing outside the crate can reach is noise that implies a stability promise the crate does not make.

`pub` is reserved for the day a library target exists. Until then a lint warns on unreachable `pub`.

Fields are private by default, with accessors where reading is genuinely needed. A newtype whose invariant is enforced in its constructor and whose field is public has no invariant.

## Module form

Post-2018 form: a module lives in `foo.rs`, and its children in a sibling `foo/` directory. `mod.rs` is not used.

The reason is mechanical: a tree of `mod.rs` files gives every open editor tab the same name.

## Error stack

Errors are typed per layer and converge on one application error:

| Layer         | Type             | Purpose                                                                                        |
| ------------- | ---------------- | ---------------------------------------------------------------------------------------------- |
| Domain        | `DomainError`    | Invariant violations in pure code — an invalid identifier, a malformed path component          |
| Configuration | `ConfigError`    | Layer-walk failures that keep the file, the key, and the underlying `io::Error`                |
| Adapter       | `std::io::Error` | Adapters return the system's own error; the caller that knows the operation names it           |
| Application   | `AppError`       | What every layer converges on: one `ErrorKind` and one diagnostic. Owns the exit-code mapping. |

Rules:

- Every error carries the concrete value involved — the path, the key, the account. An error that says a file could not be read without saying which file has failed at its one job.
- Conversions between layers are derived where the mapping is total, and written by hand where context must be added. A conversion that discards context is worse than none.
- `AppError` carries a closed `ErrorKind` with no catch-all member. The kind owns the code and the published spelling, so an error without a code is unconstructible rather than caught by a test. A parallel variant list beside the kind would only give the two of them a way to drift apart; see [exit codes](./exit-codes.md).
- A boxed trait-object error is never a return type in this crate. It erases exactly the type information the exit-code mapping needs.
- A service takes each dependency it reaches the outside world through as a trait parameter, not as a context to fish it out of. That is what lets a test drive a resolution ladder against a fake instead of a real tree, and it is the reason the ports exist.
- The entry point holds no logic. It connects to process globals, installs logging, calls the fallible program, and converts the outcome; the fallible program takes what it needs as parameters and reads no global. The conversion is ordered report, flush, then exit ([ADR-0080](../decisions/ADR-0080-order-the-boundary-as-report-flush-exit.md)), so the record naming the failure reaches the log before the sink is joined.
- `main` returns `std::process::ExitCode`, and `std::process::exit` is not called — it skips destructors, and the non-blocking log sink is flushed by one. Reproducing a child's signal death is the single exception, because re-raising does not return; it lives in the entry point and nowhere else. Every code the process can produce is owned by one enum, hand-rolled rather than taken from a crate. See [ADR-0035](../decisions/ADR-0035-convert-the-typed-error-to-a-code-once.md) and [exit codes](./exit-codes.md).

## Panics

`unwrap` and `expect` are lint-warned. Every use needs a comment saying why the case is impossible.

The sanctioned exceptions:

| Case                                                                    | Why                                                   |
| ----------------------------------------------------------------------- | ----------------------------------------------------- |
| A regular expression or other constant parsed from a literal at startup | The input is compiled in; failure is a build-time bug |
| A lock poisoned by another thread's panic                               | The program is already failing                        |
| A slice index proved in range by an immediately preceding check         | The proof is local and visible                        |
| Test code                                                               | A panic is how a test fails                           |

Everything else returns a typed error. In particular, a missing file, a malformed configuration, a failed system call, and absent input are all ordinary conditions, and panicking on any of them turns a diagnosable error into a stack trace.

`panic!` appears nowhere outside test code. `unsafe` is forbidden crate-wide.

## Types

Parse, don't validate. Convert unvalidated input into a type that cannot be invalid, once, at the boundary. Downstream code then takes the validated type and needs no defensive checks. A function taking `&str` where it means an account identifier has pushed validation onto every caller.

Newtypes for identifiers. Account identifiers and profile names, paths with meaning, and anything else where passing the wrong string type-checks but misbehaves. Validation lives in the constructor.

`FromStr` for anything parsed from a flag or a file, so the parser and the configuration loader share one implementation.

Prefer borrowed parameters. Take `&str` and `&Path` rather than `String` and `PathBuf` unless ownership is genuinely needed.

OS strings at the boundary. Anything that came from or is going to the operating system — arguments, environment values, paths — stays an OS string until something genuinely needs text. Converting to UTF-8 for convenience is how the passthrough contract breaks; see [the CLI surface](./cli-surface.md).

Enums over boolean pairs. Two related booleans admit a state that cannot happen. An enum does not.

Prefer `LazyLock` to a runtime-initialized global, and prefer passing the value to either.

## Documentation comments

Every module begins with a `//!` header stating what it is for and what it is not for. The second half is the useful one: it is what stops a module from accumulating everything adjacent to its topic.

Every `pub(crate)` item has a doc comment. A function's comment says what it does and what it returns on failure; it does not restate the signature.

Comments inside a function are for rationale: why a surprising boundary exists, which invariant must hold, which external constraint forced the shape. A comment narrating what the next line does should be deleted, or replaced by a better name. The test is whether deleting the comment would leave a future maintainer confused; if not, it is noise.

Where a decision record governs the code, the comment names it. That is the link that keeps rationale findable from the code.

## Lints

| Setting                             | Level  | Why                                                   |
| ----------------------------------- | ------ | ----------------------------------------------------- |
| `unsafe_code`                       | forbid | Nothing here needs it                                 |
| `unused_must_use`                   | deny   | A discarded result is a swallowed error               |
| `unreachable_pub`                   | warn   | Keeps the `pub(crate)` default honest                 |
| Clippy `all`, `pedantic`, `nursery` | warn   | Broad by default; suppress individually with a reason |
| Clippy `unwrap_used`, `expect_used` | warn   | Enforces the panic policy                             |

Suppressing a lint requires a scoped attribute with a comment. A crate-wide suppression needs a decision record.

Formatting and lint enforcement are in [testing and quality](./testing-and-quality.md).

## Implementation-slice boundaries

Every implementation slice inherits these boundaries even when its prose omits them. A slice may narrow its scope but cannot weaken a durable contract.

| Boundary                                                                                                                                                          | Exact owner                                                                                                                                                                                                                                                                      | Rejecting check                                         |
| ----------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------- |
| Preserve argument order, count, bytes, and empty arguments as OS strings; preserve the `--` escape.                                                               | [CLI passthrough contract](./cli-surface.md#passthrough-contract)                                                                                                                                                                                                                | Golden argv and sentinel tests                          |
| Resolve every wrapper-owned artifact through the XDG owner; never use a direct home or personal path.                                                             | [XDG storage](./xdg-storage.md)                                                                                                                                                                                                                                                  | Path and security tests plus the self-containment sweep |
| Keep parse-shape in `cli/`, convert once in `commands/`, and keep validated runtime-shape in `domain/`.                                                           | [Architecture §§ The two shapes and Module roles](../explanation/architecture.md#the-two-shapes)                                                                                                                                                                                 | Import and boundary review                              |
| Keep I/O in adapters behind ports, keep domain pure, and keep dependency direction `main → commands → services → adapters/domain`.                                | [Architecture § Module roles](../explanation/architecture.md#module-roles)                                                                                                                                                                                                       | Boundary lint                                           |
| Build one immutable `AppContext`, read no ambient environment deep in services, and add no other global state.                                                    | [Architecture § AppContext](../explanation/architecture.md#appcontext)                                                                                                                                                                                                           | Unit and review checks                                  |
| Keep typed layer errors converging on a closed `AppError`; render and map only at the entry point; use no catch-all, boxed return error, or `std::process::exit`. | [Error stack](#error-stack) and [exit codes](./exit-codes.md)                                                                                                                                                                                                                    | Exhaustive matrix test                                  |
| Keep terminal output behind the one output writer and preserve standard-output and standard-error ownership.                                                      | [Logging and output § The stream contract](./logging-and-output.md#the-stream-contract)                                                                                                                                                                                          | Output-ownership lint and integration assertions        |
| Keep OS-originated arguments, environment values, and paths as OS strings until text is required.                                                                 | [Types](#types)                                                                                                                                                                                                                                                                  | Non-UTF-8 argv and environment tests, environment lint  |
| Forbid crate-wide `unsafe` and production `panic!`; justify every `unwrap`, `expect`, and lint suppression locally.                                               | [Panics](#panics) and [Lints](#lints)                                                                                                                                                                                                                                            | Rust and Clippy gates                                   |
| Preserve `xtask` isolation and export only the library surface its tooling needs.                                                                                 | [Architecture § One shipped crate, plus `xtask`](../explanation/architecture.md#one-shipped-crate-plus-xtask), [ADR-0014](../decisions/ADR-0014-xtask-workspace-for-dev-tooling.md), and [Dependencies § Development tooling](./dependencies.md#development-tooling--xtask-only) | Tooling-isolation lint and manifest review              |
| Admit dependencies only through the dependency procedure; add nothing early.                                                                                      | [Dependencies § Adding a dependency](./dependencies.md#adding-a-dependency)                                                                                                                                                                                                      | `Cargo.lock`, deny, audit, machete, and review          |
| Add or update a mandatory test whenever a slice implements a documented contract; a marker or compile success alone is not closure.                               | [Testing and quality §§ Mandatory tests and The gate](./testing-and-quality.md#mandatory-tests)                                                                                                                                                                                  | Named test plus the full gate                           |
| Treat `docs/` as normative pre-implementation design; when a slice conflicts, correct the slice, or change the owner and governing ADR first.                     | [AGENTS.md § Decisions](../../AGENTS.md#decisions)                                                                                                                                                                                                                               | Owner and decision reconciliation                       |
| At slice completion, run its acceptance commands and the repository gate before changing milestone status; do not infer completion from files existing.           | [Milestones](../plan/milestones.md) and [Testing and quality § The gate](./testing-and-quality.md#the-gate)                                                                                                                                                                      | Acceptance results plus the full gate                   |

Before editing, resolve every cited ADR to a current status and read all `Governed by` owners. During editing, stop when work would cross a listed boundary without an owning contract. Before marking done, run the named checks, read them against slice acceptance, move the milestone line to `## closed` with its new status, and delete `tasks.md`.

## Further reading

- [Rust API Guidelines: naming](https://rust-lang.github.io/api-guidelines/naming.html)
- [Rust API Guidelines: predictability](https://rust-lang.github.io/api-guidelines/predictability.html)
- [ripgrep's flag pipeline](https://github.com/BurntSushi/ripgrep/tree/master/crates/core/flags)
- [Parse, don't validate](https://lexi-lambda.github.io/blog/2019/11/05/parse-don-t-validate/)
- [The newtype pattern](https://rust-unofficial.github.io/patterns/patterns/behavioural/newtype.html)
- [Error handling in Rust](https://burntsushi.net/rust-error-handling/)
- [Visibility and privacy](https://doc.rust-lang.org/reference/visibility-and-privacy.html)
